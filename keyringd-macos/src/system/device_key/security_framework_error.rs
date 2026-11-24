/// Owned diagnostics from Security.framework. CFError cannot cross threads, so
/// retain its domain and numeric code as well as its description before returning.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SecurityFrameworkError {
    #[error("Security.framework OSStatus operation failed: {0}")]
    OsStatus(#[from] security_framework::base::Error),
    #[error("{domain} error {code}: {description}")]
    Native {
        domain: String,
        code: isize,
        description: String,
    },
}

impl From<core_foundation::error::CFError> for SecurityFrameworkError {
    fn from(error: core_foundation::error::CFError) -> Self {
        Self::Native {
            domain: error.domain().to_string(),
            code: error.code(),
            description: error.to_string(),
        }
    }
}
