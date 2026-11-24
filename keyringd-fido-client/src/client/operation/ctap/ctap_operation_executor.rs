use keyringd_fido::AuthenticatorTransport;

use crate::{
    AuthenticatorCapabilities, AuthenticatorCapabilityError, AuthenticatorRequirements, CtapClient,
    CtapClientError, FidoClientError, GetAssertionOperationContext, GetAssertionsResult,
    MakeCredentialOperationContext, MakeCredentialResult, OperationStatus,
};

use super::CtapUserVerification;
use super::response_validation::*;

/// Execution part requesting registration/authentication from the connected CTAP authenticator. Responsible for the execution of commands and the assembly of the results from the response.
/// Preparation for PIN and user verification is delegated to CTAPUSERVERIFICATION.
/// It maintains the candidate's communication path and ability, and also deals with cancellations and connection terminations.
pub(crate) struct CtapOperationExecutor {
    client: CtapClient,
    usb_device_id: Option<crate::usb::UsbHidDeviceId>,
    capabilities: AuthenticatorCapabilities,
    transport: AuthenticatorTransport,
    status_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<crate::CtapTransportStatus>>,
}

impl CtapOperationExecutor {
    #[cfg(test)]
    pub(crate) fn new(
        channel: impl crate::CtapTransport + 'static,
        transport: AuthenticatorTransport,
        capabilities: AuthenticatorCapabilities,
    ) -> Self {
        Self {
            usb_device_id: None,
            client: CtapClient::new(Box::new(channel)),
            transport,
            capabilities,
            status_receiver: None,
        }
    }

    pub(in crate::client::operation) fn usb_device_id(
        &self,
    ) -> Option<&crate::usb::UsbHidDeviceId> {
        self.usb_device_id.as_ref()
    }

    pub(in crate::client::operation) fn with_usb_device_id(
        mut self,
        id: crate::usb::UsbHidDeviceId,
    ) -> Self {
        self.usb_device_id = Some(id);
        self
    }

    pub(in crate::client::operation) async fn select(
        &mut self,
        operation: &super::super::operation_context::OperationContext<'_>,
    ) -> Result<(), FidoClientError> {
        if let Some(receiver) = &mut self.status_receiver {
            while receiver.try_recv().is_ok() {}
        }
        let result = operation
            .run_with_status(
                &self.transport,
                OperationStatus::AuthenticatorSelectionRequired {
                    operation_id: operation.id,
                    transport: self.transport.clone(),
                },
                self.status_receiver.as_mut(),
                self.client.selection(
                    self.capabilities
                        .version()
                        .expect("connected authenticator has a supported CTAP version"),
                ),
            )
            .await
            .and_then(|result| result.map_err(FidoClientError::from));
        match result {
            Ok(()) => Ok(()),
            Err(error) => self.finish(Err(error)).await,
        }
    }

    pub(crate) async fn make_credential(
        mut self,
        context: &MakeCredentialOperationContext<'_>,
    ) -> Result<MakeCredentialResult, FidoClientError> {
        let mut request = context.prepared.ctap_request().clone();
        let operation = &context.operation;
        let stop = operation.cancellation.clone();
        let handler = operation.handler;
        let operation_id = operation.id;
        request = self
            .capabilities
            .constrain_make_credential_request(request)
            .map_err(FidoClientError::from)?;
        let large_blob = super::ctap_large_blob::CtapLargeBlob::new(
            context
                .prepared
                .options
                .extensions()
                .and_then(|e| e.large_blob()),
            self.capabilities.info(),
            self.capabilities.maximum_message_size(),
        );
        large_blob.configure_make(&mut request)?;
        let mut status_receiver = self.status_receiver.take();
        // Notifications that are being checked are not carried over to the UI of the operation.
        if let Some(receiver) = &mut status_receiver {
            while receiver.try_recv().is_ok() {}
        }
        let execute = async {
            CtapUserVerification::new(
                &mut self.client,
                &self.capabilities,
                &self.transport,
                context.prepared.user_verification(),
            )
            .map_err(FidoClientError::from)?
            .apply_make_user_verification(operation, &mut request)
            .await?;
            handler
                .on_operation_status_changed(OperationStatus::TransportSelected {
                    operation_id,
                    transport: self.transport.clone(),
                })
                .await?;
            handler
                .on_operation_status_changed(
                    operation.user_confirmation_status(
                        &self.transport,
                        request
                            .options()
                            .and_then(|options| options.get("uv"))
                            .copied()
                            .unwrap_or(false),
                    ),
                )
                .await?;
            self.capabilities
                .ensure_message_size(
                    (request.to_cbor_bytes())
                        .map_err(FidoClientError::protocol)?
                        .len()
                        + 1,
                )
                .map_err(FidoClientError::from)?;
            let mut response = operation
                .run_with_status(
                    &self.transport,
                    operation.user_confirmation_status(
                        &self.transport,
                        request
                            .options()
                            .and_then(|options| options.get("uv"))
                            .copied()
                            .unwrap_or(false),
                    ),
                    status_receiver.as_mut(),
                    self.client.make_credential(&request),
                )
                .await?;
            if matches!(
                response,
                Err(CtapClientError::Status(
                    keyringd_fido::CtapStatusCode::PinUvAuthTokenRequired
                ))
            ) && CtapUserVerification::new(
                &mut self.client,
                &self.capabilities,
                &self.transport,
                context.prepared.user_verification(),
            )
            .map_err(FidoClientError::from)?
            .apply_make_required_pin_uv_auth(operation, &mut request)
            .await?
            {
                self.capabilities
                    .ensure_message_size(
                        (request.to_cbor_bytes())
                            .map_err(FidoClientError::protocol)?
                            .len()
                            + 1,
                    )
                    .map_err(FidoClientError::from)?;
                response = operation
                    .run_with_status(
                        &self.transport,
                        operation.user_confirmation_status(
                            &self.transport,
                            request
                                .options()
                                .and_then(|options| options.get("uv"))
                                .copied()
                                .unwrap_or(false),
                        ),
                        status_receiver.as_mut(),
                        self.client.make_credential(&request),
                    )
                    .await?;
            }
            response.map_err(FidoClientError::from)
        };
        let response = tokio::select! {
            biased;
            _ = stop.cancelled() => Err(FidoClientError::Cancelled),
            response = execute => response,
        };
        let response = self.finish(response).await?;
        let large_blob_output = large_blob.registration_output(&response);
        let mut result = super::make_credential_result(
            response,
            context.prepared.client_data.as_bytes(),
            self.transport,
        )?
        .with_usb_device_id(self.usb_device_id);
        if let Some(output) = large_blob_output {
            result = result.with_large_blob(output);
        }
        Ok(result)
    }

    pub(crate) async fn get_assertions(
        mut self,
        context: &GetAssertionOperationContext<'_>,
    ) -> Result<GetAssertionsResult, FidoClientError> {
        let mut request = context.prepared.ctap_request().clone();
        let operation = &context.operation;
        let stop = operation.cancellation.clone();
        let handler = operation.handler;
        let operation_id = operation.id;
        request = self
            .capabilities
            .constrain_get_assertion_request(request)
            .map_err(FidoClientError::from)?;
        let large_blob = super::ctap_large_blob::CtapLargeBlob::new(
            context
                .prepared
                .options
                .extensions()
                .and_then(|e| e.large_blob()),
            self.capabilities.info(),
            self.capabilities.maximum_message_size(),
        );
        large_blob.configure_get(&mut request)?;
        let mut status_receiver = self.status_receiver.take();
        // Notifications that are being checked are not carried over to the UI of the operation.
        if let Some(receiver) = &mut status_receiver {
            while receiver.try_recv().is_ok() {}
        }
        let execute = async {
            let mut token = CtapUserVerification::new(
                &mut self.client,
                &self.capabilities,
                &self.transport,
                context.prepared.user_verification(),
            )
            .map_err(FidoClientError::from)?
            .apply_get_user_verification(
                operation,
                &mut request,
                large_blob.needs_write_token() && self.capabilities.supports_permission_tokens(),
            )
            .await?;
            handler
                .on_operation_status_changed(OperationStatus::TransportSelected {
                    operation_id,
                    transport: self.transport.clone(),
                })
                .await?;
            handler
                .on_operation_status_changed(
                    operation.user_confirmation_status(
                        &self.transport,
                        request
                            .options()
                            .and_then(|options| options.get("uv"))
                            .copied()
                            .unwrap_or(false),
                    ),
                )
                .await?;
            self.capabilities
                .ensure_message_size(
                    (request.to_cbor_bytes())
                        .map_err(FidoClientError::protocol)?
                        .len()
                        + 1,
                )
                .map_err(FidoClientError::from)?;
            let mut responses = operation
                .run_with_status(
                    &self.transport,
                    operation.user_confirmation_status(
                        &self.transport,
                        request
                            .options()
                            .and_then(|options| options.get("uv"))
                            .copied()
                            .unwrap_or(false),
                    ),
                    status_receiver.as_mut(),
                    self.client.get_assertions(
                        &request,
                        self.capabilities
                            .version()
                            .expect("connected authenticator has a supported CTAP version"),
                    ),
                )
                .await?;
            if matches!(
                responses,
                Err(CtapClientError::Status(
                    keyringd_fido::CtapStatusCode::PinUvAuthTokenRequired
                ))
            ) {
                let retry_token = CtapUserVerification::new(
                    &mut self.client,
                    &self.capabilities,
                    &self.transport,
                    context.prepared.user_verification(),
                )
                .map_err(FidoClientError::from)?
                .apply_get_required_pin_uv_auth(
                    operation,
                    &mut request,
                    large_blob.needs_write_token()
                        && self.capabilities.supports_permission_tokens(),
                )
                .await?;
                if let Some(retry_token) = retry_token {
                    token = Some(retry_token);
                    self.capabilities
                        .ensure_message_size(
                            (request.to_cbor_bytes())
                                .map_err(FidoClientError::protocol)?
                                .len()
                                + 1,
                        )
                        .map_err(FidoClientError::from)?;
                    responses = operation
                        .run_with_status(
                            &self.transport,
                            operation.user_confirmation_status(
                                &self.transport,
                                request
                                    .options()
                                    .and_then(|options| options.get("uv"))
                                    .copied()
                                    .unwrap_or(false),
                            ),
                            status_receiver.as_mut(),
                            self.client.get_assertions(
                                &request,
                                self.capabilities
                                    .version()
                                    .expect("connected authenticator has a supported CTAP version"),
                            ),
                        )
                        .await?;
                }
            }
            let responses = responses.map_err(FidoClientError::from)?;
            for response in &responses {
                validate_authenticator_extension_outputs(
                    response.authenticator_data().extensions(),
                )?;
                validate_authenticator_extension_outputs(response.unsigned_extension_outputs())?;
                validate_discoverable_user(
                    context.prepared.ctap_request().allow_list(),
                    response.user(),
                )?;
            }
            let result = GetAssertionsResult::from_ctap(
                responses,
                context.prepared.client_data.clone(),
                self.transport.clone(),
                request.allow_list(),
            )?;
            let mut result = context.prepared.validate_result(result)?;
            large_blob
                .complete(&mut self.client, result.responses_mut(), token.as_ref())
                .await;
            Ok(result)
        };
        let result = tokio::select! {
            biased;
            _ = stop.cancelled() => Err(FidoClientError::Cancelled),
            result = execute => result,
        };
        let result = self
            .finish(result)
            .await?
            .with_usb_device_id(self.usb_device_id);
        Ok(result)
    }

    /// Reads capabilities and rejects unsuitable candidates before a ceremony begins.
    pub(crate) async fn connect(
        channel: Box<dyn crate::CtapTransport>,
        transport: AuthenticatorTransport,
        requirements: &AuthenticatorRequirements,
        request: crate::AuthenticatorRequest<'_>,
        stop: tokio_util::sync::CancellationToken,
        status_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<crate::CtapTransportStatus>>,
    ) -> Result<CtapOperationExecutor, FidoClientError> {
        let mut client = CtapClient::new(channel);
        let info = tokio::select! {
            biased;
            _ = stop.cancelled() => Err(FidoClientError::Cancelled),
            result = client.get_info() => result.map_err(FidoClientError::from),
        };
        let info = match info {
            Ok(info) => info,
            Err(error) => {
                cleanup_after_failure(&mut client, &error).await;
                return Err(error);
            }
        };
        let capabilities = AuthenticatorCapabilities::from_get_info(info);
        let suitability = capabilities
            .check_requirements(requirements, request)
            .and_then(|()| {
                CtapUserVerification::check_requirements(
                    &capabilities,
                    &requirements.user_verification,
                )
            });
        match suitability {
            Ok(()) => {}
            Err(error) => {
                let error = match error {
                    error @ AuthenticatorCapabilityError::PinChangeRequired => {
                        FidoClientError::pin_change_required(error)
                    }
                    _ => FidoClientError::UnsupportedRequirement(error.into()),
                };
                cleanup_after_failure(&mut client, &error).await;
                return Err(error);
            }
        }
        Ok(Self {
            usb_device_id: None,
            client,
            transport,
            capabilities,
            status_receiver,
        })
    }

    pub(in crate::client::operation) async fn finish<T>(
        &mut self,
        result: Result<T, FidoClientError>,
    ) -> Result<T, FidoClientError> {
        match result {
            Err(error) => {
                cleanup_after_failure(&mut self.client, &error).await;
                Err(error)
            }
            Ok(value) => {
                self.client.close().await.map_err(FidoClientError::from)?;
                Ok(value)
            }
        }
    }
}

async fn cleanup_after_failure(client: &mut CtapClient, error: &FidoClientError) {
    let cleanup = if matches!(
        error,
        FidoClientError::Cancelled | FidoClientError::TimedOut
    ) {
        client.cancel().await
    } else {
        client.close().await
    };
    if let Err(cleanup_error) = cleanup {
        tracing::warn!(
            ?error,
            ?cleanup_error,
            "CTAP cleanup failed after an operation failure; retaining the operation error"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::operation::OperationContext;
    use crate::test_support::MockCtapTransport;
    use crate::{ClientData, OperationId};
    use keyringd_fido::ctap::{
        AuthenticatorGetAssertionRequest, AuthenticatorGetAssertionResponse,
        AuthenticatorGetInfoResponse,
    };
    use keyringd_fido::webauthn::{
        PublicKeyCredentialDescriptor, PublicKeyCredentialRequestOptions, PublicKeyCredentialType,
    };
    use keyringd_fido::{
        AuthenticatorData, CtapCommand, CtapResponse, CtapStatusCode, UserVerificationRequirement,
    };
    use sha2::Digest;

    #[tokio::test]
    async fn resolves_ctap20_credential_with_count_one_from_the_filtered_allow_list() {
        let credential =
            PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, vec![1]);
        let too_long =
            PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, vec![2; 17]);
        let options = PublicKeyCredentialRequestOptions::new(vec![3])
            .with_relying_party_id("example.com")
            .with_allow_credentials(vec![credential.clone(), too_long.clone()])
            .with_user_verification(UserVerificationRequirement::Discouraged);
        let prepared = crate::client::PreparedAssertion::new(options, &|challenge: &[u8]| {
            Ok(ClientData::from_bytes(challenge.to_vec()))
        })
        .unwrap();
        let response = AuthenticatorGetAssertionResponse::new(
            None,
            AuthenticatorData::new("example.com", 1, 7).unwrap(),
            vec![1],
        )
        .with_number_of_credentials(1);
        let payload = response.to_cbor_bytes().unwrap();
        let mut channel = MockCtapTransport::new();
        channel
            .expect_transmit()
            .times(1)
            .withf(|request| {
                request.command() == CtapCommand::GetAssertion
                    && AuthenticatorGetAssertionRequest::from_cbor_bytes(request.payload())
                        .unwrap()
                        .allow_list()
                        .unwrap()
                        .len()
                        == 1
            })
            .return_once(move |_| {
                Box::pin(async move { Ok(CtapResponse::new(CtapStatusCode::Ok, payload)) })
            });
        channel
            .expect_close()
            .times(1)
            .returning(|| Box::pin(async { Ok(()) }));
        let capabilities = AuthenticatorCapabilities::from_get_info(
            AuthenticatorGetInfoResponse::new(vec!["FIDO_2_0".into()], [0; 16])
                .with_max_credential_id_length(1),
        );
        let operation = OperationContext {
            id: OperationId::new(1),
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(5),
            cancellation: Default::default(),
            handler: &(),
        };
        let context = GetAssertionOperationContext::new(&prepared, operation, None, None);
        let result =
            CtapOperationExecutor::new(channel, AuthenticatorTransport::usb(), capabilities)
                .get_assertions(&context)
                .await
                .unwrap();
        assert_eq!(result.responses()[0].credential(), &credential);
        assert_eq!(result.responses()[0].number_of_credentials(), Some(1));
        let selected = result.select(None).unwrap();
        assert_eq!(selected.credential(), &credential);
        assert_eq!(selected.response().authenticator_data().sign_count(), 7);
    }
    #[tokio::test]
    async fn reads_large_blob_from_unsigned_outputs_without_another_assertion() {
        use keyringd_fido::webauthn::{
            AuthenticationExtensionsAuthenticatorOutputs, AuthenticationExtensionsClientInputs,
            LargeBlobInput, LargeBlobOutput,
        };
        use std::io::Write;
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(b"saved registration").unwrap();
        let compressed = encoder.finish().unwrap();
        // The extension is deliberately unsigned; including it in authData would break SSH SK signatures.
        let mut bytes = vec![
            0xa2,
            0x64,
            b'b',
            b'l',
            b'o',
            b'b',
            0x40 + compressed.len() as u8,
        ];
        bytes.extend_from_slice(&compressed);
        bytes.push(0x6c);
        bytes.extend_from_slice(b"originalSize");
        bytes.push(18);
        let output = AuthenticationExtensionsAuthenticatorOutputs::default().with_value(
            "largeBlob",
            keyringd_fido::EncodedCborValue::from_canonical_bytes(bytes).unwrap(),
        );
        let credential =
            PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, vec![1]);
        let options = PublicKeyCredentialRequestOptions::new(vec![3])
            .with_relying_party_id("example.com")
            .with_allow_credentials(vec![credential.clone()])
            .with_user_verification(UserVerificationRequirement::Discouraged)
            .with_extensions(
                AuthenticationExtensionsClientInputs::default()
                    .with_large_blob(LargeBlobInput::request_read()),
            );
        let prepared = crate::client::PreparedAssertion::new(options, &|challenge: &[u8]| {
            Ok(ClientData::from_bytes(challenge.to_vec()))
        })
        .unwrap();
        let payload = AuthenticatorGetAssertionResponse::new(
            None,
            AuthenticatorData::new("example.com", 1, 0).unwrap(),
            vec![1],
        )
        .with_unsigned_extension_outputs(output)
        .to_cbor_bytes()
        .unwrap();
        let mut channel = MockCtapTransport::new();
        channel
            .expect_transmit()
            .times(1)
            .withf(|request| {
                request.command() == CtapCommand::GetAssertion
                    && AuthenticatorGetAssertionRequest::from_cbor_bytes(request.payload())
                        .unwrap()
                        .extensions()
                        .unwrap()
                        .values()
                        .contains_key("largeBlob")
            })
            .return_once(move |_| {
                Box::pin(async move { Ok(CtapResponse::new(CtapStatusCode::Ok, payload)) })
            });
        channel
            .expect_close()
            .once()
            .returning(|| Box::pin(async { Ok(()) }));
        let capabilities = AuthenticatorCapabilities::from_get_info(
            AuthenticatorGetInfoResponse::new(vec!["FIDO_2_1".into()], [0; 16])
                .with_extensions(vec!["largeBlob".into()]),
        );
        let operation = OperationContext {
            id: OperationId::new(1),
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(5),
            cancellation: Default::default(),
            handler: &(),
        };
        let context = GetAssertionOperationContext::new(&prepared, operation, None, None);
        let result =
            CtapOperationExecutor::new(channel, AuthenticatorTransport::usb(), capabilities)
                .get_assertions(&context)
                .await
                .unwrap();
        assert_eq!(
            result.responses()[0].large_blob(),
            Some(&LargeBlobOutput::Read(Some(b"saved registration".to_vec())))
        );
        assert!(
            result.responses()[0]
                .authenticator_data()
                .extensions()
                .is_none()
        );
    }
    #[tokio::test]
    async fn writes_large_blob_with_the_assertion_token_and_no_second_uv_or_up() {
        use crate::ctap::test_support::TestPinUvKeyAgreement;
        use keyringd_fido::ctap::{
            AuthenticatorClientPinRequest, AuthenticatorClientPinResponse,
            AuthenticatorLargeBlobsRequest, AuthenticatorLargeBlobsResponse, ClientPinSubcommand,
            LargeBlobArray, PinUvAuthPermissions,
        };
        use keyringd_fido::webauthn::{
            AuthenticationExtensionsClientInputs, LargeBlobInput, LargeBlobOutput,
        };
        for (capacity, flags, expected_commands) in [(17, 5, 4), (4096, 5, 5), (4096, 1, 3)] {
            let agreement = TestPinUvKeyAgreement::new(crate::PinUvAuthProtocol::Two);
            let auth_token =
                crate::PinUvAuthToken::new(crate::PinUvAuthProtocol::Two, vec![0x5a; 32]).unwrap();
            let credential =
                PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, vec![1]);
            let options = PublicKeyCredentialRequestOptions::new(vec![3])
                .with_relying_party_id("example.com")
                .with_allow_credentials(vec![credential.clone()])
                .with_user_verification(UserVerificationRequirement::Required)
                .with_extensions(
                    AuthenticationExtensionsClientInputs::default().with_large_blob(
                        LargeBlobInput::request_write(b"saved registration".to_vec()),
                    ),
                );
            let prepared = crate::client::PreparedAssertion::new(options, &|challenge: &[u8]| {
                Ok(ClientData::from_bytes(challenge.to_vec()))
            })
            .unwrap();
            let mut channel = MockCtapTransport::new();
            channel
                .expect_transmit()
                .times(expected_commands)
                .returning(move |wire| {
                    let payload = match wire.command() {
                        CtapCommand::ClientPin => {
                            let pin =
                                AuthenticatorClientPinRequest::from_cbor_bytes(wire.payload())
                                    .unwrap();
                            match pin.subcommand() {
                                ClientPinSubcommand::GetKeyAgreement => {
                                    AuthenticatorClientPinResponse::new()
                                        .with_key_agreement(agreement.public_key().unwrap())
                                        .to_cbor_bytes()
                                        .unwrap()
                                }
                                ClientPinSubcommand::GetPinUvAuthTokenUsingUvWithPermissions => {
                                    assert_eq!(
                                        pin.permissions(),
                                        Some(
                                            PinUvAuthPermissions::GET_ASSERTION
                                                .union(PinUvAuthPermissions::LARGE_BLOB_WRITE)
                                        )
                                    );
                                    assert_eq!(pin.relying_party_id(), Some("example.com"));
                                    let (_, shared) =
                                        agreement.establish(pin.key_agreement().unwrap()).unwrap();
                                    AuthenticatorClientPinResponse::new()
                                        .with_pin_uv_auth_token(
                                            shared.encrypt(&[0x5a; 32]).unwrap(),
                                        )
                                        .to_cbor_bytes()
                                        .unwrap()
                                }
                                _ => panic!("unexpected extra authorization"),
                            }
                        }
                        CtapCommand::GetAssertion => {
                            let request =
                                AuthenticatorGetAssertionRequest::from_cbor_bytes(wire.payload())
                                    .unwrap();
                            assert_eq!(
                                request.pin_uv_auth_param(),
                                Some(
                                    auth_token
                                        .authenticate(request.client_data_hash().as_bytes())
                                        .unwrap()
                                        .as_slice()
                                )
                            );
                            assert!(
                                request
                                    .extensions()
                                    .unwrap()
                                    .values()
                                    .contains_key("largeBlobKey")
                            );
                            AuthenticatorGetAssertionResponse::new(
                                None,
                                AuthenticatorData::new("example.com", flags, 0).unwrap(),
                                vec![1],
                            )
                            .with_large_blob_key(vec![9; 32])
                            .to_cbor_bytes()
                            .unwrap()
                        }
                        CtapCommand::LargeBlobs => {
                            assert_eq!(flags, 5, "unverified assertion accessed largeBlob storage");
                            match AuthenticatorLargeBlobsRequest::from_cbor_bytes(wire.payload())
                                .unwrap()
                            {
                                AuthenticatorLargeBlobsRequest::Read { .. } => {
                                    AuthenticatorLargeBlobsResponse::new(
                                        LargeBlobArray::default().to_bytes().unwrap(),
                                    )
                                    .to_cbor_bytes()
                                    .unwrap()
                                }
                                AuthenticatorLargeBlobsRequest::Write {
                                    offset,
                                    data,
                                    pin_uv_auth,
                                    ..
                                } => {
                                    let mut message = vec![0xff; 32];
                                    message.extend_from_slice(&[0x0c, 0]);
                                    message.extend_from_slice(&(offset as u32).to_le_bytes());
                                    message.extend_from_slice(&sha2::Sha256::digest(&data));
                                    assert_eq!(
                                        pin_uv_auth.unwrap().0,
                                        auth_token.authenticate(&message).unwrap()
                                    );
                                    Vec::new()
                                }
                            }
                        }
                        _ => panic!("unexpected command"),
                    };
                    Box::pin(async move { Ok(CtapResponse::new(CtapStatusCode::Ok, payload)) })
                });
            channel
                .expect_close()
                .once()
                .returning(|| Box::pin(async { Ok(()) }));
            let capabilities = AuthenticatorCapabilities::from_get_info(
                AuthenticatorGetInfoResponse::new(vec!["FIDO_2_1".into()], [0; 16])
                    .with_extensions(vec!["largeBlobKey".into()])
                    .with_options(std::collections::BTreeMap::from([
                        ("uv".into(), true),
                        ("pinUvAuthToken".into(), true),
                        ("largeBlobs".into(), true),
                    ]))
                    .with_pin_uv_auth_protocols(vec![2])
                    .with_max_serialized_large_blob_array(capacity),
            );
            let operation = OperationContext {
                id: OperationId::new(1),
                deadline: std::time::Instant::now() + std::time::Duration::from_secs(5),
                cancellation: Default::default(),
                handler: &(),
            };
            let context = GetAssertionOperationContext::new(&prepared, operation, None, None);
            let result =
                CtapOperationExecutor::new(channel, AuthenticatorTransport::usb(), capabilities)
                    .get_assertions(&context)
                    .await;
            if flags == 1 {
                let error = result.unwrap_err();
                assert_eq!(
                    std::error::Error::source(&error)
                        .and_then(|e| e.downcast_ref::<crate::ResponseValidationError>()),
                    Some(&crate::ResponseValidationError::UserVerificationMissing)
                );
                continue;
            }
            let result = result.unwrap();
            assert_eq!(
                result.responses()[0].large_blob(),
                Some(&LargeBlobOutput::Written(capacity > 17))
            );
        }
    }
}
