use crate::AuthenticatorCapabilities;
use crate::AuthenticatorCapabilityError;
use crate::AuthenticatorRequest;
use crate::AuthenticatorRequirements;
use crate::SelectedUserVerification;
use crate::client::operation::ctap::select_user_verification;
use keyringd_fido::ctap::AuthenticatorGetInfoResponse;
use keyringd_fido::webauthn::PublicKeyCredentialParameters;
use std::collections::BTreeMap;

pub(crate) fn request() -> keyringd_fido::ctap::AuthenticatorGetAssertionRequest {
    keyringd_fido::ctap::AuthenticatorGetAssertionRequest::new(
        "example.com",
        keyringd_fido::ctap::ClientDataHash::new([0; 32]),
    )
}

pub(crate) fn info() -> AuthenticatorGetInfoResponse {
    AuthenticatorGetInfoResponse::new(vec!["FIDO_2_1".into()], [0; 16])
        .with_extensions(vec!["credProtect".into()])
        .with_options(BTreeMap::from([
            ("rk".into(), true),
            ("clientPin".into(), true),
            ("pinUvAuthToken".into(), true),
        ]))
        .with_min_pin_length(4)
        .with_max_message_size(2048)
        .with_pin_uv_auth_protocols(vec![1, 2])
        .with_algorithms(vec![PublicKeyCredentialParameters::es256()])
}

pub(crate) fn check_candidate(
    info: &AuthenticatorGetInfoResponse,
    requirements: &AuthenticatorRequirements,
    request: AuthenticatorRequest<'_>,
) -> Result<(AuthenticatorCapabilities, SelectedUserVerification), AuthenticatorCapabilityError> {
    let capabilities = AuthenticatorCapabilities::from_get_info(info.clone());
    capabilities.check_requirements(requirements, request)?;
    let selected = select_user_verification(info, &requirements.user_verification)?;
    Ok((capabilities, selected))
}
