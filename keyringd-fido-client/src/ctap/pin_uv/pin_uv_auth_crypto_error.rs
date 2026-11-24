/// Errors in PIN/UV key exchange, encryption, and authentication processing.
#[derive(Debug, thiserror::Error)]
pub enum PinUvAuthCryptoError {
    #[error("invalid peer key agreement public key")]
    InvalidPeerPublicKey(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("invalid key or input length for PIN/UV protocol {protocol}")]
    InvalidLength { protocol: u64 },
    #[error("PIN/UV key derivation failed")]
    KeyDerivation,
    #[error("PIN/UV encryption failed")]
    Encryption,
    #[error("PIN/UV decryption failed")]
    Decryption,
}
