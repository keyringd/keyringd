use keyringd_core::model::device_key::DeviceKeyAlgorithm;

/// Errors from tss-esapi TPM operations (Linux).
#[derive(Debug, thiserror::Error)]
pub(crate) enum EsapiError {
    #[error("TPM error: {0}")]
    Tss(#[from] tss_esapi::Error),
    #[error("permission denied accessing /dev/tpmrm0")]
    AccessDenied(#[source] std::io::Error),
    #[error("unsupported algorithm: {0:?}")]
    UnsupportedAlgorithm(DeviceKeyAlgorithm),
    #[error("invalid key handle")]
    InvalidKeyHandle(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("invalid EC point from TPM")]
    InvalidEcPoint(#[source] Option<Box<dyn std::error::Error + Send + Sync>>),
    #[error("digest length rejected by TPM")]
    InvalidDigestLength,
    #[error("TPM password entry was cancelled")]
    PasswordCancelled,
    #[error("TPM password interaction failed")]
    Interaction(#[from] std::io::Error),
    #[error("could not derive TPM authorization value")]
    PasswordDerivation,
    #[error("TPM blocking task failed: {0}")]
    BlockingTask(#[from] tokio::task::JoinError),
}
