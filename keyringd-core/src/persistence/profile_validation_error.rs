#[derive(Debug, thiserror::Error)]
pub enum ProfileValidationError {
    #[error("unsupported profiles schema version: {0}")]
    UnsupportedSchemaVersion(u32),
    #[error("{0}")]
    InvalidProfile(&'static str),
    #[error("profiles state contains a duplicate profile ID")]
    DuplicateId,
}
