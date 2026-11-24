use crate::AuthenticatorTransport;
use crate::ctap::AuthenticatorGetInfoResponse;
use crate::ctap::PublicKeyCredentialParameters;
use std::collections::BTreeMap;
pub(crate) fn minimal() -> AuthenticatorGetInfoResponse {
    AuthenticatorGetInfoResponse::new(vec!["FIDO_2_1".to_owned()], [0x11; 16])
        .with_options(BTreeMap::from([
            ("rk".to_owned(), true),
            ("clientPin".to_owned(), false),
        ]))
        .with_max_message_size(1200)
        .with_pin_uv_auth_protocols(vec![2, 1])
        .with_transports(vec![AuthenticatorTransport::usb()])
        .with_algorithms(vec![PublicKeyCredentialParameters::es256()])
        .with_min_pin_length(4)
        .with_remaining_discoverable_credentials(7)
}
