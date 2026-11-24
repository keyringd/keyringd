#[derive(Debug, thiserror::Error)]
pub enum ModelAuthenticatorError {
    #[error("credential not found")]
    CredentialNotFound,
    #[error("signing failed: {0}")]
    SigningFailed(#[from] ecdsa::Error),
}
