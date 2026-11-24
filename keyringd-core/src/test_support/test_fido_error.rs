#[derive(Debug, thiserror::Error)]
pub enum TestFidoError {
    #[error("credential not found")]
    CredentialNotFound,
    #[error("signing failed: {0}")]
    SigningFailed(#[from] p256::ecdsa::Error),
    #[error("test authenticator response could not be encoded: {0}")]
    Encoding(String),
}
