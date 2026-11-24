#[derive(Debug, thiserror::Error)]
pub(crate) enum RecoveryError {
    #[error("invalid DER signature")]
    InvalidSignature(#[source] p256::ecdsa::Error),
    #[error("recovery failed for both candidates")]
    RecoveryFailed(#[source] Option<Box<dyn std::error::Error + Send + Sync>>),
    #[error("neither candidate verified the second signature")]
    AmbiguousRecovery,
}
