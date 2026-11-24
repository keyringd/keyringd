use crate::test_support::MockCtapTransport;
use crate::*;

use keyringd_fido::ctap::PinUvAuthPermissions;
use keyringd_fido::ctap::{AuthenticatorGetAssertionRequest, AuthenticatorMakeCredentialRequest};

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use keyringd_fido::{CtapCommand, CtapResponse, CtapStatusCode};

pub(crate) fn built_in_uv_transport(
    authenticator: Arc<keyringd_test_authenticator::ModelCtapAuthenticator>,
    pin_uv_authenticator: crate::ctap::test_support::TestPinUvKeyAgreement,
    client_pin_subcommands: Arc<Mutex<Vec<keyringd_fido::ctap::ClientPinSubcommand>>>,
    mut uv_failures_remaining: usize,
    uv_failure_status: keyringd_fido::CtapStatusCode,
    uv_retries: u64,
) -> MockCtapTransport {
    let mut transport = MockCtapTransport::new();
    let cancel_model = authenticator.clone();
    transport.expect_transmit().returning(move |wire_request| {
        let command = wire_request.command();
        let request = wire_request.payload();
        let encoded: Result<Vec<u8>, crate::CtapTransportError> = (|| {
            if command != keyringd_fido::CtapCommand::ClientPin {
                return authenticator
                    .transmit(command, request)
                    .map_err(crate::CtapTransportError::new);
            }
            let request =
                keyringd_fido::ctap::AuthenticatorClientPinRequest::from_cbor_bytes(request)
                    .unwrap();
            let subcommand = request.subcommand();
            client_pin_subcommands.lock().unwrap().push(subcommand);
            if subcommand
            == keyringd_fido::ctap::ClientPinSubcommand::GetPinUvAuthTokenUsingUvWithPermissions
            && uv_failures_remaining > 0
        {
            uv_failures_remaining -= 1;
            return Ok(vec![uv_failure_status.as_byte()]);
        }
            let response = match subcommand {
            keyringd_fido::ctap::ClientPinSubcommand::GetKeyAgreement => {
                keyringd_fido::ctap::AuthenticatorClientPinResponse::new()
                    .with_key_agreement(pin_uv_authenticator.public_key().unwrap())
            }
            keyringd_fido::ctap::ClientPinSubcommand::GetUvRetries => {
                keyringd_fido::ctap::AuthenticatorClientPinResponse::new()
                    .with_uv_retries(uv_retries)
            }
            keyringd_fido::ctap::ClientPinSubcommand::SetPin => {
                keyringd_fido::ctap::AuthenticatorClientPinResponse::new()
            }
            keyringd_fido::ctap::ClientPinSubcommand::GetPinRetries => {
                keyringd_fido::ctap::AuthenticatorClientPinResponse::new()
                    .with_pin_retries(8)
            }
            keyringd_fido::ctap::ClientPinSubcommand::GetPinUvAuthTokenUsingUvWithPermissions
            | keyringd_fido::ctap::ClientPinSubcommand::GetPinUvAuthTokenUsingPinWithPermissions => {
                assert_eq!(
                    request.permissions(),
                    Some(PinUvAuthPermissions::MAKE_CREDENTIAL)
                );
                assert_eq!(request.relying_party_id(), Some("ssh:keyringd"));
                let (_, shared) = pin_uv_authenticator
                    .establish(request.key_agreement().unwrap())
                    .unwrap();
                keyringd_fido::ctap::AuthenticatorClientPinResponse::new()
                    .with_pin_uv_auth_token(
                        shared.encrypt(&[0x5a; 32]).unwrap(),
                    )
            }
            _ => panic!("unexpected ClientPIN subcommand {subcommand:?}"),
        };
            let mut encoded = vec![keyringd_fido::CtapStatusCode::Ok.as_byte()];
            encoded.extend_from_slice(&response.to_cbor_bytes().unwrap());
            Ok(encoded)


        })();
        let response = encoded.and_then(|encoded| CtapResponse::from_bytes(&encoded).map_err(crate::CtapTransportError::new));
        Box::pin(async move { response })
    });
    transport.expect_cancel().returning(move || {
        cancel_model.cancel();
        Box::pin(async { Ok(()) })
    });
    transport
        .expect_close()
        .returning(|| Box::pin(async { Ok(()) }));
    transport
}

pub(crate) fn puat_required_once_transport(
    authenticator: std::sync::Arc<keyringd_test_authenticator::ModelCtapAuthenticator>,
    get_assertion_attempts: Arc<AtomicUsize>,
    pin_uv_authenticator: crate::ctap::test_support::TestPinUvKeyAgreement,
    reject_first_get_assertion: bool,
    expected_permissions: PinUvAuthPermissions,
) -> MockCtapTransport {
    let mut transport = MockCtapTransport::new();
    let cancel_model = authenticator.clone();
    let mut client_pin_step = 0;
    transport.expect_transmit().returning(move |wire_request| {
        let command = wire_request.command();
        let request = wire_request.payload();
        let encoded: Result<Vec<u8>, crate::CtapTransportError> = (|| {
            if command == keyringd_fido::CtapCommand::ClientPin {
                let request =
                    keyringd_fido::ctap::AuthenticatorClientPinRequest::from_cbor_bytes(request)
                        .unwrap();
                let response = match client_pin_step {
                    0 => {
                        assert_eq!(
                            request.subcommand(),
                            keyringd_fido::ctap::ClientPinSubcommand::GetKeyAgreement
                        );
                        keyringd_fido::ctap::AuthenticatorClientPinResponse::new()
                            .with_key_agreement(pin_uv_authenticator.public_key().unwrap())
                    }
                    1 => {
                        assert_eq!(
                        request.subcommand(),
                        keyringd_fido::ctap::ClientPinSubcommand::GetPinUvAuthTokenUsingUvWithPermissions
                    );
                        assert_eq!(request.permissions(), Some(expected_permissions));
                        assert_eq!(request.relying_party_id(), Some("ssh:keyringd"));
                        let (_, shared) = pin_uv_authenticator
                            .establish(request.key_agreement().unwrap())
                            .unwrap();
                        keyringd_fido::ctap::AuthenticatorClientPinResponse::new()
                            .with_pin_uv_auth_token(shared.encrypt(&[0x5a; 32]).unwrap())
                    }
                    _ => panic!("unexpected ClientPIN request"),
                };
                client_pin_step += 1;
                let mut encoded = vec![keyringd_fido::CtapStatusCode::Ok.as_byte()];
                encoded.extend_from_slice(&response.to_cbor_bytes().unwrap());
                return Ok(encoded);
            }
            if command == keyringd_fido::CtapCommand::GetAssertion {
                let attempt = get_assertion_attempts.fetch_add(1, Ordering::SeqCst);
                if attempt == 0 && reject_first_get_assertion {
                    return Ok(vec![
                        keyringd_fido::CtapStatusCode::PinUvAuthTokenRequired.as_byte(),
                    ]);
                }
                let request = AuthenticatorGetAssertionRequest::from_cbor_bytes(request).unwrap();
                assert!(request.pin_uv_auth_param().is_some());
                assert_eq!(request.pin_uv_auth_protocol(), Some(2));
                assert_ne!(
                    request.options().and_then(|options| options.get("uv")),
                    Some(&true)
                );
            }
            if command == keyringd_fido::CtapCommand::MakeCredential {
                let request = AuthenticatorMakeCredentialRequest::from_cbor_bytes(request).unwrap();
                assert!(request.pin_uv_auth_param().is_some());
                assert_eq!(request.pin_uv_auth_protocol(), Some(2));
                assert_ne!(
                    request.options().and_then(|options| options.get("uv")),
                    Some(&true)
                );
            }
            let encoded = authenticator
                .transmit(command, request)
                .map_err(crate::CtapTransportError::new)?;
            Ok(encoded)


        })();
        let response = encoded.and_then(|encoded| CtapResponse::from_bytes(&encoded).map_err(crate::CtapTransportError::new));
        Box::pin(async move { response })
    });
    transport.expect_cancel().returning(move || {
        cancel_model.cancel();
        Box::pin(async { Ok(()) })
    });
    transport
        .expect_close()
        .returning(|| Box::pin(async { Ok(()) }));
    transport
}

pub(crate) fn selection_transport(
    inner: MockCtapTransport,
    commands: Arc<Mutex<Vec<CtapCommand>>>,
    touch: Arc<tokio::sync::Notify>,
    cancelled: Arc<AtomicBool>,
    version: keyringd_fido::ctap::CtapVersion,
    selection_status: CtapStatusCode,
) -> MockCtapTransport {
    let inner = Arc::new(tokio::sync::Mutex::new(inner));
    let mut transport = MockCtapTransport::new();
    let operation = inner.clone();
    transport.expect_transmit().returning(move |request| {
        commands.lock().unwrap().push(request.command());
        let selection_command = if version == keyringd_fido::ctap::CtapVersion::Fido20 {
            CtapCommand::GetAssertion
        } else {
            CtapCommand::Selection
        };
        if request.command() == selection_command {
            if selection_command == CtapCommand::Selection {
                assert!(request.payload().is_empty());
            } else {
                let probe =
                    AuthenticatorGetAssertionRequest::from_cbor_bytes(request.payload()).unwrap();
                assert!(probe.rp_id().starts_with("keyringd-selection-"));
                assert!(probe.rp_id().ends_with(".invalid"));
                assert_eq!(probe.allow_list().unwrap().len(), 1);
                assert_eq!(probe.allow_list().unwrap()[0].id().len(), 32);
                assert_eq!(probe.options().unwrap().len(), 1);
                assert_eq!(probe.options().unwrap().get("up"), Some(&true));
                assert!(probe.pin_uv_auth_param().is_none());
                assert!(probe.pin_uv_auth_protocol().is_none());
                assert!(probe.extensions().is_none());
            }
            let touch = touch.clone();
            Box::pin(async move {
                touch.notified().await;
                Ok(CtapResponse::new(selection_status, Vec::new()))
            })
        } else {
            let operation = operation.clone();
            let request = request.clone();
            Box::pin(async move { operation.lock().await.transmit(&request).await })
        }
    });
    transport.expect_cancel().returning(move || {
        cancelled.store(true, Ordering::SeqCst);
        let inner = inner.clone();
        Box::pin(async move { inner.lock().await.cancel().await })
    });
    transport
        .expect_close()
        .returning(|| Box::pin(async { Ok(()) }));
    transport
}
