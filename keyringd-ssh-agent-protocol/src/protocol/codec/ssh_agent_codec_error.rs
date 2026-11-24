#[derive(Debug, thiserror::Error)]
pub enum SshAgentCodecError {
    #[error("SSH value is empty")]
    EmptyMessage,
    #[error("SSH value is truncated at {field}")]
    Truncated { field: &'static str },
    #[error("SSH value contains {trailing_bytes} trailing bytes")]
    TrailingData { trailing_bytes: usize },
    #[error("SSH {field} is not valid UTF-8")]
    InvalidUtf8 { field: &'static str },
    #[error("unsupported SSH {field}")]
    Unsupported { field: &'static str },
    #[error("invalid SSH {field}: {reason}")]
    InvalidValue {
        field: &'static str,
        reason: &'static str,
    },
    #[error("SSH {field} length does not fit the wire format")]
    LengthOverflow { field: &'static str },
    #[error("SSH Agent identity count does not fit the wire format")]
    TooManyIdentities,
    #[error("SSH certificate validity end must be later than its start")]
    InvalidValidityPeriod,
    #[error("SSH certificate signature is malformed")]
    InvalidSignature,
}
