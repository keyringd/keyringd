use super::{
    AuthenticatorBroker, FidoClientError, FidoInteractionHandler, GetAssertionOperationContext,
    GetAssertionsResult, InternalAuthenticator, MakeCredentialOperationContext,
    MakeCredentialResult, OperationManager, TransportSelection, backend,
};
use keyringd_fido::webauthn::LargeBlobOutput;
use keyringd_fido::webauthn::LargeBlobSupport;

#[cfg(test)]
use crate::SystemHybridChannelConnector;

use std::time::{Duration, Instant};

use keyringd_fido::webauthn::PublicKeyCredentialCreationOptions;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// The entrance to which WebAuthn registration/authentication is performed. Summarize input preparation, operation life management, and run of authentication device candidates.
/// The route is determined by the options' Hints or the dialog handler, and the previous operation selection is not preserved.
pub struct FidoClient<H, A = ()> {
    pub(crate) operation_manager: OperationManager<H>,
    pub(crate) broker: AuthenticatorBroker,
    pub(crate) internal_authenticator: A,
}

impl<H> FidoClient<H>
where
    H: FidoInteractionHandler,
{
    pub fn new(interaction_handler: H) -> Self {
        Self {
            operation_manager: OperationManager::new(interaction_handler),
            broker: AuthenticatorBroker::default(),
            internal_authenticator: (),
        }
    }
}

impl<H: FidoInteractionHandler, A: InternalAuthenticator> FidoClient<H, A> {
    pub fn with_usb_backend(mut self, backend: backend::UsbBackend) -> Self {
        self.broker.set_usb_backend(backend);
        self
    }

    pub fn with_hybrid_connector(mut self, connector: crate::SystemHybridChannelConnector) -> Self {
        self.broker
            .set_hybrid_connector(std::sync::Arc::new(connector));
        self
    }

    pub fn with_internal_authenticator<B: InternalAuthenticator>(
        self,
        authenticator: B,
    ) -> FidoClient<H, B> {
        FidoClient {
            operation_manager: self.operation_manager,
            broker: self.broker,
            internal_authenticator: authenticator,
        }
    }

    pub fn internal_authenticator(&self) -> &A {
        &self.internal_authenticator
    }

    pub async fn make_credential(
        &self,
        options: PublicKeyCredentialCreationOptions,
        context: super::MakeCredentialContext<'_>,
    ) -> Result<MakeCredentialResult, FidoClientError> {
        let super::MakeCredentialContext {
            client_data_provider,
            usb_device_id,
        } = context;
        let selection = TransportSelection::from_hints(options.hints())?;
        if usb_device_id.is_some() && selection == Some(TransportSelection::Internal) {
            return Err(FidoClientError::UnsupportedRequirement(
                "an internal authenticator cannot use a USB device ID".into(),
            ));
        }
        let deadline = self.deadline(options.timeout());
        let prepared = super::PreparedMakeCredential::new(options, client_data_provider)?;
        let large_blob_required = prepared
            .options
            .extensions()
            .and_then(|e| e.large_blob())
            .is_some_and(|input| input.support() == Some(&LargeBlobSupport::Required));
        let result = self
            .operation_manager
            .run_operation(deadline, |operation| async move {
                let context = MakeCredentialOperationContext::new(
                    &prepared,
                    operation,
                    selection,
                    usb_device_id,
                );
                self.broker
                    .make_credential(context, &self.internal_authenticator)
                    .await
            })
            .await?;
        if large_blob_required
            && !matches!(
                result.credential().client_extension_results().large_blob(),
                Some(LargeBlobOutput::Supported(true))
            )
        {
            return Err(FidoClientError::UnsupportedRequirement(
                "credential does not support largeBlob".into(),
            ));
        }
        if result.credential().response().transports().is_empty() {
            return Err(FidoClientError::NoAuthenticator);
        }
        Ok(result)
    }

    pub async fn get_assertions(
        &self,
        options: keyringd_fido::webauthn::PublicKeyCredentialRequestOptions,
        context: super::GetAssertionsContext<'_>,
    ) -> Result<GetAssertionsResult, FidoClientError> {
        let super::GetAssertionsContext {
            client_data_provider,
            usb_device_id,
            user_presence,
        } = context;
        let selection = TransportSelection::from_hints(options.hints())?;
        if usb_device_id.is_some() && selection == Some(TransportSelection::Internal) {
            return Err(FidoClientError::UnsupportedRequirement(
                "an internal authenticator cannot use a USB device ID".into(),
            ));
        }
        let deadline = self.deadline(options.timeout());
        let mut prepared = super::PreparedAssertion::new(options, client_data_provider)?;
        if !user_presence {
            prepared = prepared.with_user_presence(false);
        }
        self.operation_manager
            .run_operation(deadline, |operation| async move {
                let context = GetAssertionOperationContext::new(
                    &prepared,
                    operation,
                    selection,
                    usb_device_id,
                );
                let result = self
                    .broker
                    .get_assertions(context, &self.internal_authenticator)
                    .await?;
                prepared.validate_result(result)
            })
            .await
    }

    pub fn cancel(&self, operation_id: crate::OperationId) -> bool {
        self.operation_manager.cancel(operation_id)
    }

    pub(crate) fn deadline(&self, timeout: Option<u32>) -> Instant {
        let requested = timeout
            .map(|milliseconds| Duration::from_millis(u64::from(milliseconds)))
            .unwrap_or(DEFAULT_TIMEOUT);
        Instant::now() + requested
    }
}

#[cfg(test)]
mod tests {

    use crate::client::test_support::{
        LifecycleCalls, OsBackendCalls, lifecycle_handler, pending_os_backend,
    };

    use crate::{ClientData, ResponseValidationError};
    use std::sync::Arc;

    use keyringd_fido::ctap::AuthenticatorGetAssertionResponse;
    use keyringd_fido::webauthn::{
        AuthenticatorData, AuthenticatorTransport, PublicKeyCredentialDescriptor,
        PublicKeyCredentialParameters, PublicKeyCredentialRequestOptions,
        PublicKeyCredentialRpEntity, PublicKeyCredentialType, PublicKeyCredentialUserEntity,
    };

    use super::*;
    use crate::client::test_support::MockInternalAuthenticator;
    use keyringd_fido::ctap::ClientDataHash;
    use keyringd_fido::webauthn::UserVerificationRequirement;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::test_support::assert_fido_error;

    keyringd_testkit::scenario! {
        group "FidoClient::deadline" {
            case "Uses the client default only when the caller omits the timeout" {
                why "This native API policy is independent of browser WebAuthn timeout range correction."
                given "a client with the fixed two-minute default timeout" {
                    let client = FidoClient::new(crate::test_support::usb_only_handler()).with_usb_backend(crate::backend::UsbBackend::Hid(Arc::new(crate::test_support::empty_usb_backend()))).with_hybrid_connector(SystemHybridChannelConnector::default());
                }
                when "deadlines are calculated for an omitted timeout and a fifteen-minute caller timeout" {
                    let default_deadline = client.deadline(None);
                    let requested_deadline = client.deadline(Some(900_000));
                    let now = Instant::now();
                    let default_remaining = default_deadline.saturating_duration_since(now);
                    let requested_remaining = requested_deadline.saturating_duration_since(now);
                }
                then "the omitted timeout uses two minutes and the fifteen-minute timeout is used unchanged" {
                    assert!(default_remaining <= Duration::from_secs(120));
                    assert!(default_remaining > Duration::from_millis(119_900));
                    assert!(requested_remaining <= Duration::from_secs(900));
                    assert!(requested_remaining > Duration::from_millis(899_900));
                }
            }
        }
        group "GetAssertionsResult::validate" {
            case "Rejects duplicate assertion credentials" {
                why "The multi-assertion Rust result must reject duplicate credential IDs so callers cannot mistake repeated transport output for distinct accounts."
                spec "https://www.w3.org/TR/webauthn-3/#sctn-getAssertion"
                given "Prepare multiple Assertions with the same Credential ID" {
                    let mut raw_authenticator_data = vec![0; 32];
                    raw_authenticator_data.push(0x01);
                    raw_authenticator_data.extend_from_slice(&0u32.to_be_bytes());
                    let authenticator_data = AuthenticatorData::from_bytes(&raw_authenticator_data).unwrap();
                    let assertion = AuthenticatorGetAssertionResponse::new(Some(PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, vec![1, 2, 3])),
                        authenticator_data,
                        vec![0x30, 0],
                    )
                    .with_user(PublicKeyCredentialUserEntity::from_id(vec![9]));
                }
                when "Verify the results of the operation" {
                    let error = GetAssertionsResult::from_ctap(
                        vec![assertion.clone(), assertion],
                        ClientData::from_bytes(vec![4]),
                        AuthenticatorTransport::usb(), None).unwrap()
                    .validate()
                    .expect_err("duplicate credential IDs must fail");
                }
                then "Verify that the results match FidoClientError::Protocol and ResponseValidationError::DuplicateCredential" {
                    assert!(matches!(&error, FidoClientError::Protocol(_)));
                    let source = std::error::Error::source(&error).expect("protocol error source");
                    assert_eq!(
                        source.downcast_ref::<ResponseValidationError>(),
                        Some(&ResponseValidationError::DuplicateCredential)
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "FidoClient::make_credential" {
            case "Applies shared requirements to the OS USB backend" {
                why "Every backend must pass through the same high-level option validation; this architectural boundary prevents backend-specific policy bypasses."
                spec "https://www.w3.org/TR/webauthn-3/#sctn-createCredential"
                spec "https://www.w3.org/TR/webauthn-3/#sctn-discover-from-external-source"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                given "Prepare an OS USB Backend and Credential Option that do not meet your requirements" {
                    let handler = Arc::new(LifecycleCalls::default());
                    let backend = Arc::new(OsBackendCalls::default());
                    let client = FidoClient::new(lifecycle_handler(handler)).with_usb_backend(crate::backend::UsbBackend::Os(pending_os_backend(backend.clone()))).with_hybrid_connector(crate::backend::SystemHybridChannelConnector::default());
                }
                when "Call ClientData::from_bytes" {
                    let creation_options = PublicKeyCredentialCreationOptions::new(
                        PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                        PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                        vec![2],
                        vec![PublicKeyCredentialParameters::new(
                            PublicKeyCredentialType::PublicKey,
                            keyringd_fido::webauthn::CoseAlgorithmIdentifier::ES256,
                        )],
                    );
                    let creation_parameters = std::sync::Arc::new(|_: &[u8]| Ok(ClientData::from_bytes(vec![3])));
                }
                then "Verify that the results match FidoClientError::UnsupportedRequirement" {
                    assert_fido_error!(
                        client
                            .make_credential(creation_options, keyringd_fido_client::MakeCredentialContext::new((creation_parameters).as_ref()))
                            .await,
                        FidoClientError::UnsupportedRequirement(_)
                    );
                    let request_options = PublicKeyCredentialRequestOptions::new(vec![1]);
                    let request_parameters = std::sync::Arc::new(|_: &[u8]| Ok(ClientData::from_bytes(vec![4])));
                    assert_fido_error!(
                        client.get_assertions(request_options, keyringd_fido_client::GetAssertionsContext::new((request_parameters).as_ref())).await,
                        FidoClientError::UnsupportedRequirement(_)
                    );
                    assert_eq!(*backend.active.lock().unwrap(), None);
                    assert_eq!(*backend.make_credential_client_data.lock().unwrap(), None);
                    assert_eq!(*backend.get_assertions_client_data.lock().unwrap(), None);
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "FidoClient (Internal)" {
            case "Registers through the connected internal authenticator" {
                async: tokio;
                verify "makeCredential uses CTAP data and reports platform attachment and internal transport" {
                    use keyringd_fido::webauthn::{AuthenticatorAttachment, AuthenticatorSelectionCriteria, PublicKeyCredentialHint, ResidentKeyRequirement};
                    use keyringd_fido::ctap::AuthenticatorMakeCredentialResponse;
                    let model = keyringd_test_authenticator::ModelCtapAuthenticator::new();
                    let mut authenticator = MockInternalAuthenticator::new();
                    authenticator.expect_make_credential().times(1)
                        .withf(|request, _| request.client_data_hash() == &ClientDataHash::hash(b"registration data"))
                        .returning(move |request, _| {
                            let encoded = model.transmit(keyringd_fido::CtapCommand::MakeCredential, &request.to_cbor_bytes().unwrap()).unwrap();
                            assert_eq!(encoded[0], 0);
                            let response = AuthenticatorMakeCredentialResponse::from_cbor_bytes(&encoded[1..]).unwrap();
                            Box::pin(async { Ok(response) })
                        });
                    let client = FidoClient::new(()).with_internal_authenticator(authenticator);
                    let options = PublicKeyCredentialCreationOptions::new(
                        PublicKeyCredentialRpEntity::new("Example").with_id("example.org"),
                        PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                        vec![2], vec![PublicKeyCredentialParameters::es256()],
                    ).with_hints(vec![PublicKeyCredentialHint::client_device()])
                    .with_authenticator_selection(AuthenticatorSelectionCriteria::default()
                        .with_authenticator_attachment(AuthenticatorAttachment::Platform)
                        .with_resident_key(ResidentKeyRequirement::Required));
                    let result = client.make_credential(options, keyringd_fido_client::MakeCredentialContext::new(&|_: &[u8]| Ok(ClientData::from_bytes(b"registration data".to_vec())))).await.unwrap();
                    assert_eq!(result.credential().authenticator_attachment(), Some(&AuthenticatorAttachment::Platform));
                    assert_eq!(result.credential().response().transports(), &[AuthenticatorTransport::internal()]);
                    assert_eq!(result.credential().response().client_data_json(), b"registration data");
                }
            }
            case "Connects an arbitrary authenticator using CTAP and retains client data once" {
                async: tokio;
                verify "the library has no Keyringd RP key format UI context or signature algorithm assumptions" {
                    let credential = PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, vec![9]);
                    let expected_credential = credential.clone();
                    let mut authenticator = MockInternalAuthenticator::new();
                    authenticator.expect_get_assertion().times(1).withf(move |request, _| {
                        request.rp_id() == "example.org"
                            && request.client_data_hash() == &ClientDataHash::hash(b"external client data")
                            && request.allow_list() == Some([expected_credential.clone()].as_slice())
                            && request.options().is_some_and(|options| options.get("uv") == Some(&true) && !options.contains_key("up"))
                    }).returning(move |_, _| { let credential = credential.clone(); Box::pin(async move { Ok(AuthenticatorGetAssertionResponse::new(Some(credential), AuthenticatorData::new("example.org", 5, 19).unwrap(), vec![0x80; 64])) }) });
                    let calls = Arc::new(AtomicUsize::new(0));
                    let count = Arc::clone(&calls);
                    let provider = move |challenge: &[u8]| {
                        assert_eq!(challenge, b"challenge");
                        count.fetch_add(1, Ordering::SeqCst);
                        Ok(ClientData::from_bytes(b"external client data".to_vec()))
                    };
                    let options = PublicKeyCredentialRequestOptions::new(b"challenge".to_vec()).with_relying_party_id("example.org")
                        .with_hints(vec![keyringd_fido::webauthn::PublicKeyCredentialHint::client_device()])
                        .with_allow_credentials(vec![PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, vec![9])])
                        .with_user_verification(UserVerificationRequirement::Required);
                    let result = FidoClient::new(()).with_internal_authenticator(authenticator).get_assertions(options, keyringd_fido_client::GetAssertionsContext::new(&provider)).await.unwrap();
                    assert_eq!(calls.load(Ordering::SeqCst), 1);
                    assert_eq!(result.client_data().as_bytes(), b"external client data");
                    assert_eq!(result.responses().first().unwrap().authenticator_data().sign_count(), 19);
                    assert_eq!(result.responses().first().unwrap().signature(), &[0x80; 64]);
                }
            }
            case "Applies user presence requirements independently to each operation" {
                async: tokio;
                verify "disabling presence for one operation does not weaken the next operation" {
                    let credential = PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, vec![1]);
                    let returned = credential.clone();
                    let mut authenticator = MockInternalAuthenticator::new();
                    authenticator.expect_get_assertion().times(1).withf(|request, _| request.options().is_some_and(|options| options.get("up") == Some(&false)))
                        .returning(move |_, _| { let returned = returned.clone(); Box::pin(async move { Ok(AuthenticatorGetAssertionResponse::new(Some(returned), AuthenticatorData::new("example.net", 0, 0).unwrap(), vec![1; 64])) }) });
                    let returned = credential.clone();
                    authenticator.expect_get_assertion().times(1).withf(|request, _| request.options().is_none_or(|options| options.get("up") != Some(&false)))
                        .returning(move |_, _| { let returned = returned.clone(); Box::pin(async move { Ok(AuthenticatorGetAssertionResponse::new(Some(returned), AuthenticatorData::new("example.net", 1, 0).unwrap(), vec![1; 64])) }) });
                    let options = PublicKeyCredentialRequestOptions::new(vec![1]).with_hints(vec![keyringd_fido::webauthn::PublicKeyCredentialHint::client_device()]).with_relying_party_id("example.net").with_allow_credentials(vec![credential]);
                    let client = FidoClient::new(()).with_internal_authenticator(authenticator);
                    let result = client.get_assertions(options.clone(), keyringd_fido_client::GetAssertionsContext::new(&|_: &[u8]| Ok(ClientData::from_bytes(vec![2]))).with_user_presence(false)).await.unwrap();
                    assert_eq!(result.responses().first().unwrap().authenticator_data().flags(), 0);
                    let result = client.get_assertions(options, keyringd_fido_client::GetAssertionsContext::new(&|_: &[u8]| Ok(ClientData::from_bytes(vec![2])))).await.unwrap();
                    assert_eq!(result.responses().first().unwrap().authenticator_data().flags(), 1);
                }
            }
            case "Preserves authenticator cancellation errors" {
                async: tokio;
                verify "a cancelled backend operation never becomes a successful assertion" {
                    let mut authenticator = MockInternalAuthenticator::new();
                    authenticator.expect_get_assertion().returning(|_, _| Box::pin(async { Err(FidoClientError::Cancelled) }));
                    let options = PublicKeyCredentialRequestOptions::new(vec![1]).with_hints(vec![keyringd_fido::webauthn::PublicKeyCredentialHint::client_device()]).with_relying_party_id("example.org");
                    assert!(matches!(FidoClient::new(()).with_internal_authenticator(authenticator).get_assertions(options, keyringd_fido_client::GetAssertionsContext::new(&|_: &[u8]| Ok(ClientData::from_bytes(vec![2])))).await, Err(FidoClientError::Cancelled)));
                }
            }
            case "Keeps internal operations busy until cancellation cleanup finishes" {
                why "Native authentication may outlive a cancellation or deadline; releasing the operation early permits overlapping prompts."
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                verify "both explicit cancellation and timeout notify the backend and await cleanup" {
                    for timed_out in [false, true] {
                        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
                        let cleanup = Arc::new(tokio::sync::Semaphore::new(0));
                        let mut authenticator = MockInternalAuthenticator::new();
                        let observed = cancelled.clone();
                        let finished = cleanup.clone();
                        authenticator.expect_get_assertion().times(1).returning(move |_, cancellation| {
                            let observed = observed.clone();
                            let finished = finished.clone();
                            Box::pin(async move {
                                cancellation.cancelled().await;
                                observed.store(true, Ordering::SeqCst);
                                finished.acquire().await.unwrap().forget();
                                Err(FidoClientError::Cancelled)
                            })
                        });
                        let events = Arc::new(LifecycleCalls::default());
                        let client = FidoClient::new(lifecycle_handler(events.clone())).with_internal_authenticator(authenticator);
                        let options = PublicKeyCredentialRequestOptions::new(vec![1])
                            .with_relying_party_id("example.org")
                            .with_hints(vec![keyringd_fido::webauthn::PublicKeyCredentialHint::client_device()])
                            .with_timeout(if timed_out { 5 } else { 1000 });
                        let provider = |_: &[u8]| Ok(ClientData::from_bytes(vec![2]));
                        let operation = client.get_assertions(options.clone(), keyringd_fido_client::GetAssertionsContext::new(&provider));
                        tokio::pin!(operation);
                        assert!(futures_util::poll!(operation.as_mut()).is_pending());
                        let id = events.events.lock().unwrap()[0].1;
                        if !timed_out { assert!(client.cancel(id)); }
                        tokio::select! {
                            _ = &mut operation => panic!("operation completed before cleanup"),
                            _ = async { while !cancelled.load(Ordering::SeqCst) { tokio::task::yield_now().await; } } => {}
                        }
                        assert!(matches!(client.get_assertions(options, keyringd_fido_client::GetAssertionsContext::new(&provider)).await, Err(FidoClientError::Busy)));
                        assert_eq!(events.events.lock().unwrap().len(), 1);
                        cleanup.add_permits(1);
                        let result = operation.await;
                        assert!(if timed_out { matches!(result, Err(FidoClientError::TimedOut)) } else { matches!(result, Err(FidoClientError::Cancelled)) });
                        assert_eq!(events.events.lock().unwrap().last().unwrap(), &("backend_finished", id));
                    }
                }
            }
        }
    }
}
