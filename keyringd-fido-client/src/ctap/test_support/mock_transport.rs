use super::TestPinUvKeyAgreement;
use crate::test_support::MockCtapTransport;
use keyringd_fido::CtapResponse;
use keyringd_fido::ctap::{
    AuthenticatorClientPinRequest, AuthenticatorClientPinResponse, ClientPinSubcommand,
    PinUvAuthPermissions,
};
use keyringd_fido::{CtapCommand, CtapStatusCode};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, atomic::Ordering};

pub(crate) fn recording_transport(
    mut responses: VecDeque<Vec<u8>>,
    commands: Arc<Mutex<Vec<CtapCommand>>>,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
) -> MockCtapTransport {
    let mut transport = MockCtapTransport::new();
    transport
        .expect_transmit()
        .times(responses.len())
        .returning(move |request| {
            commands.lock().unwrap().push(request.command());
            let response = CtapResponse::from_bytes(&responses.pop_front().unwrap())
                .map_err(crate::CtapTransportError::new);
            Box::pin(async move { response })
        });
    transport.expect_cancel().returning(move || {
        cancelled.store(true, Ordering::Relaxed);
        Box::pin(async { Ok(()) })
    });
    transport
}

fn expect_key_agreement(
    transport: &mut MockCtapTransport,
    authenticator: &TestPinUvKeyAgreement,
    sequence: &mut mockall::Sequence,
) {
    let response = CtapResponse::new(
        CtapStatusCode::Ok,
        AuthenticatorClientPinResponse::new()
            .with_key_agreement(authenticator.public_key().unwrap())
            .to_cbor_bytes()
            .unwrap(),
    );
    transport
        .expect_transmit()
        .times(1)
        .in_sequence(sequence)
        .withf(|request| {
            request.command() == CtapCommand::ClientPin
                && AuthenticatorClientPinRequest::from_cbor_bytes(request.payload())
                    .unwrap()
                    .subcommand()
                    == ClientPinSubcommand::GetKeyAgreement
        })
        .return_once(move |_| Box::pin(async move { Ok(response) }));
}

pub(crate) fn pin_transport(
    authenticator: TestPinUvKeyAgreement,
    permissioned: bool,
    using_uv: bool,
) -> MockCtapTransport {
    let mut transport = MockCtapTransport::new();
    let mut sequence = mockall::Sequence::new();
    expect_key_agreement(&mut transport, &authenticator, &mut sequence);
    transport
        .expect_transmit()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |wire_request| {
            assert_eq!(wire_request.command(), CtapCommand::ClientPin);
            let request =
                AuthenticatorClientPinRequest::from_cbor_bytes(wire_request.payload()).unwrap();
            let expected_subcommand = if using_uv {
                ClientPinSubcommand::GetPinUvAuthTokenUsingUvWithPermissions
            } else if permissioned {
                ClientPinSubcommand::GetPinUvAuthTokenUsingPinWithPermissions
            } else {
                ClientPinSubcommand::GetPinToken
            };
            assert_eq!(request.subcommand(), expected_subcommand);
            assert_eq!(
                request.permissions(),
                permissioned.then_some(PinUvAuthPermissions::GET_ASSERTION)
            );
            assert_eq!(
                request.relying_party_id(),
                permissioned.then_some("example.com")
            );
            assert_eq!(request.pin_hash_enc().is_some(), !using_uv);
            let (_, shared) = authenticator
                .establish(request.key_agreement().unwrap())
                .unwrap();
            let response = CtapResponse::new(
                CtapStatusCode::Ok,
                AuthenticatorClientPinResponse::new()
                    .with_pin_uv_auth_token(shared.encrypt(&[0x5a; 32]).unwrap())
                    .to_cbor_bytes()
                    .unwrap(),
            );
            Box::pin(async move { Ok(response) })
        });
    transport
}

pub(crate) fn set_pin_transport(authenticator: TestPinUvKeyAgreement) -> MockCtapTransport {
    let mut transport = MockCtapTransport::new();
    let mut sequence = mockall::Sequence::new();
    expect_key_agreement(&mut transport, &authenticator, &mut sequence);
    transport
        .expect_transmit()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |wire_request| {
            assert_eq!(wire_request.command(), CtapCommand::ClientPin);
            let request =
                AuthenticatorClientPinRequest::from_cbor_bytes(wire_request.payload()).unwrap();
            assert_eq!(request.subcommand(), ClientPinSubcommand::SetPin);
            let encrypted_pin = request.new_pin_enc().unwrap();
            let expected_length = match request.pin_uv_auth_protocol() {
                Some(1) => 64,
                Some(2) => 80,
                value => panic!("unexpected PIN/UV Auth protocol {value:?}"),
            };
            assert_eq!(encrypted_pin.len(), expected_length);
            let (_, shared) = authenticator
                .establish(request.key_agreement().unwrap())
                .unwrap();
            assert_eq!(
                request.pin_uv_auth_param().unwrap(),
                shared.authenticate(encrypted_pin).unwrap()
            );
            Box::pin(async { Ok(CtapResponse::new(CtapStatusCode::Ok, Vec::new())) })
        });
    transport
}
