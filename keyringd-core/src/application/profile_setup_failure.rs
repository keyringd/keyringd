use crate::application::ApplicationError;

#[derive(Debug, thiserror::Error)]
pub enum ProfileSetupFailure {
    #[error("profile setup was canceled")]
    Cancelled,
    #[error("no Passkey was found")]
    PasskeyNotFound,
    #[error("profile setup failed: {0}")]
    Failed(#[source] ApplicationError),
}

impl From<ApplicationError> for ProfileSetupFailure {
    fn from(error: ApplicationError) -> Self {
        match error {
            ApplicationError::OperationCanceled => Self::Cancelled,
            ApplicationError::PasskeyCredentialNotFound => Self::PasskeyNotFound,
            error => Self::Failed(error),
        }
    }
}
