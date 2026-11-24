/// An error indicating that the format of the HYBRID QR URI is incorrect.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum HybridQrDecodeError {
    #[error("Hybrid QR URI must start with FIDO:/")]
    InvalidScheme,
    #[error("Hybrid QR digit encoding has an invalid length")]
    InvalidDigitLength,
    #[error("Hybrid QR digit encoding contains a non-decimal character")]
    InvalidDigit,
    #[error("Hybrid QR digit chunk is outside its canonical byte range")]
    NonCanonicalDigitChunk,
    #[error("Hybrid QR payload is not valid CBOR")]
    InvalidCbor,
    #[error("Hybrid QR payload contains a duplicate or invalid field")]
    InvalidField,
    #[error("Hybrid QR payload is missing field {0}")]
    MissingField(u64),
    #[error("Hybrid QR public key is not a compressed P-256 key")]
    InvalidPublicKey,
    #[error("Hybrid QR secret must contain 16 bytes")]
    InvalidSecret,
    #[error("Hybrid QR operation hint is unsupported")]
    InvalidHint,
}
