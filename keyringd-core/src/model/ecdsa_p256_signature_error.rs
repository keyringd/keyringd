#[derive(Debug, thiserror::Error)]
pub enum EcdsaP256SignatureError {
    #[error("raw ECDSA P-256 signature must contain 64 bytes, got {actual}")]
    RawLength { actual: usize },
    #[error("invalid raw ECDSA P-256 signature")]
    RawScalars(#[source] p256::ecdsa::Error),
    #[error("invalid DER-encoded ECDSA P-256 signature")]
    Der(#[source] p256::ecdsa::Error),
}
