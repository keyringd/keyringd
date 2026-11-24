#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("invalid JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("failed to encode JSON: {0}")]
    EncodeJson(#[source] serde_json::Error),
    #[error("invalid canonical CBOR: {0}")]
    InvalidCbor(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("failed to encode canonical CBOR: {0}")]
    EncodeCbor(String),
    #[error("invalid base64url value: {0}")]
    InvalidBase64Url(#[source] base64::DecodeError),
    #[error("base64url value is not in canonical unpadded form")]
    NonCanonicalBase64Url,
}
