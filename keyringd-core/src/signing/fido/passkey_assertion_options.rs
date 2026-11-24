use crate::model::passkey::Passkey;
use keyringd_fido::webauthn::{
    PublicKeyCredentialHint, PublicKeyCredentialRequestOptions, UserVerificationRequirement,
};

/// Builds Keyringd's policy defaults before handing the request to a provider.
pub(crate) fn passkey_assertion_options(
    challenge: &[u8],
    hints: &[PublicKeyCredentialHint],
) -> PublicKeyCredentialRequestOptions {
    PublicKeyCredentialRequestOptions::new(challenge.to_vec())
        .with_timeout(300_000)
        .with_relying_party_id(Passkey::RP_ID)
        .with_user_verification(UserVerificationRequirement::Required)
        .with_hints(hints.to_vec())
}
