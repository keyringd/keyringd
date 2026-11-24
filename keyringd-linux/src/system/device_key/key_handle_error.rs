#[derive(Debug, thiserror::Error)]
pub(super) enum KeyHandleError {
    #[error("key handle too short: need {expected} bytes, got {actual}")]
    TooShort { expected: usize, actual: usize },
    #[error("unrecognized key handle format")]
    UnrecognizedFormat,
    #[error("unsupported TPM key local_authentication mode: {0}")]
    UnsupportedProtection(u8),
    #[error("invalid TPM private key blob: {0}")]
    InvalidPrivateKey(#[source] tss_esapi::Error),
    #[error("invalid TPM public key blob: {0}")]
    InvalidDevicePublicKey(#[source] tss_esapi::Error),
}
