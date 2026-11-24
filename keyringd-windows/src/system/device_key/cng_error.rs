#[derive(Debug, thiserror::Error)]
pub enum CngError {
    #[error("Windows CNG operation failed: {0}")]
    Windows(#[from] windows::core::Error),

    #[error("unsupported key algorithm: {0:?}")]
    UnsupportedAlgorithm(keyringd_core::model::device_key::DeviceKeyAlgorithm),

    #[error("invalid CNG key handle blob")]
    InvalidKeyHandle(#[source] Option<std::str::Utf8Error>),

    #[error("invalid CNG ECDSA public key blob")]
    InvalidDevicePublicKey(#[source] Option<Box<dyn std::error::Error + Send + Sync>>),

    #[allow(dead_code)]
    #[error("invalid CNG ECDSA signature")]
    InvalidSignature,
}
