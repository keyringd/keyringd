/// Errors caused by derivation of Hybrid keys, ad authentication/formal validation, and QR generation.
#[derive(Debug, thiserror::Error)]
pub enum HybridCryptoError {
    #[error("hybrid key derivation failed")]
    KeyDerivation,
    #[error("hybrid advertisement must contain exactly 20 bytes; got {0}")]
    InvalidAdvertisementLength(usize),
    #[error("hybrid advertisement authentication failed")]
    AdvertisementAuthentication,
    #[error("hybrid advertisement reserved byte is nonzero")]
    AdvertisementReservedBits,
    #[error("failed to encode hybrid QR CBOR")]
    QrEncoding,
    #[error("system clock is before the Unix epoch")]
    InvalidSystemTime,
    #[error("failed to generate a P-256 QR key")]
    InvalidQrKey,
}
