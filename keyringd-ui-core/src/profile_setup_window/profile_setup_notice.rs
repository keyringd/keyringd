use crate::text;
#[derive(Clone, PartialEq)]
pub enum ProfileSetupNotice {
    MissingName,
    Cancelled,
    PasskeyNotFound,
    NewPasskeyAfterFailedRecovery,
    OperationFailed(String),
}

impl ProfileSetupNotice {
    pub fn message(&self) -> &str {
        match self {
            Self::MissingName => text::PROFILE_SETUP_MISSING_NAME,
            Self::Cancelled => text::PROFILE_SETUP_CANCELLED,
            Self::PasskeyNotFound => text::PROFILE_SETUP_PASSKEY_NOT_FOUND,
            Self::NewPasskeyAfterFailedRecovery => text::PROFILE_SETUP_RECOVERY_WARNING,
            Self::OperationFailed(message) => message,
        }
    }
    pub fn is_error(&self) -> bool {
        !matches!(self, Self::Cancelled)
    }
}
