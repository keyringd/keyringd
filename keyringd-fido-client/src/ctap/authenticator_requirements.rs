use keyringd_fido::UserVerificationRequirement;
use keyringd_fido::webauthn::CoseAlgorithmIdentifier;

/// Capacity requirements required for selecting the authenticator. The CTAP request to be checked is received as a separate argument.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorRequirements {
    pub discoverable_credential: bool,
    pub user_verification: UserVerificationRequirement,
    pub algorithms: Vec<CoseAlgorithmIdentifier>,
    pub required_extensions: Vec<String>,
}

impl AuthenticatorRequirements {
    #[cfg(test)]
    pub fn keyringd() -> Self {
        Self {
            discoverable_credential: true,
            user_verification: UserVerificationRequirement::Preferred,
            algorithms: vec![CoseAlgorithmIdentifier::ES256],
            required_extensions: Vec::new(),
        }
    }
}
