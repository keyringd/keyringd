#[derive(Debug, thiserror::Error)]
pub enum ProfileSaveError {
    #[error("invalid profiles document: {0}")]
    InvalidDocument(#[from] super::ProfileValidationError),
    #[error("profile save failed: {0}")]
    BeforeReplace(#[from] std::io::Error),
    #[error("profile was replaced but its directory could not be synchronized: {0}")]
    AfterReplace(#[source] std::io::Error),
}

impl From<super::FileSaveError> for ProfileSaveError {
    fn from(error: super::FileSaveError) -> Self {
        match error {
            super::FileSaveError::BeforeReplace(error) => Self::BeforeReplace(error),
            super::FileSaveError::AfterReplace(error) => Self::AfterReplace(error),
        }
    }
}
