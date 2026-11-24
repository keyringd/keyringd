use keyringd_core::application::ApplicationError;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ApplicationLoadFailure {
    InvalidStoredData,
    StorageUnavailable,
    DeviceKeyUnavailable,
    InvariantViolation,
    Unexpected,
}

impl ApplicationLoadFailure {
    pub(crate) fn from_error(error: &ApplicationError) -> Self {
        match error {
            ApplicationError::ProfileRepository(error)
                if error.kind() == std::io::ErrorKind::InvalidData =>
            {
                Self::InvalidStoredData
            }
            ApplicationError::InvalidPersistedState(_) => Self::InvalidStoredData,
            ApplicationError::ProfileRepository(_)
            | ApplicationError::ProfileDurabilityUncertain(_) => Self::StorageUnavailable,
            ApplicationError::DeviceKey(_) | ApplicationError::DeviceKeyUnavailable(_) => {
                Self::DeviceKeyUnavailable
            }
            ApplicationError::InvariantViolation(_) => Self::InvariantViolation,
            ApplicationError::OperationRejected(_)
            | ApplicationError::NotFound(_)
            | ApplicationError::Conflict(_)
            | ApplicationError::InvalidAuthenticatorResponse(_)
            | ApplicationError::Passkey(_)
            | ApplicationError::PasskeyHybridTunnelConnectionTimedOut(_)
            | ApplicationError::PasskeyAuthenticatorNotFound(_)
            | ApplicationError::PasskeySecurityKeyTimedOut(_)
            | ApplicationError::PasskeyHybridConnectionTimedOut(_)
            | ApplicationError::PasskeyHybridConnectionLost(_)
            | ApplicationError::PasskeyPinInvalid(_)
            | ApplicationError::PasskeyPinChangeRequired(_)
            | ApplicationError::PasskeyRequestRejected(_)
            | ApplicationError::PasskeyCredentialNotFound
            | ApplicationError::OperationCanceled
            | ApplicationError::Certificate(_) => Self::Unexpected,
        }
    }

    pub(crate) fn log_category(&self) -> &'static str {
        match self {
            Self::InvalidStoredData => "invalid_stored_data",
            Self::StorageUnavailable => "storage_unavailable",
            Self::DeviceKeyUnavailable => "device_key_unavailable",
            Self::InvariantViolation => "invariant_violation",
            Self::Unexpected => "unexpected_application_error",
        }
    }

    pub(crate) fn user_message(&self) -> &'static str {
        match self {
            Self::InvalidStoredData => {
                "The saved profile data is invalid and could not be loaded.\n\nYour saved profiles have not been changed. Open the logs for details."
            }
            Self::StorageUnavailable => {
                "The saved profiles could not be accessed. This may be temporary.\n\nYour saved profiles have not been changed. Open the logs for details."
            }
            Self::DeviceKeyUnavailable => {
                "A Device Key required by a saved profile is unavailable.\n\nYour saved profiles have not been changed. Open the logs for details."
            }
            Self::InvariantViolation => {
                "Keyringd detected an inconsistent application state.\n\nYour saved profiles have not been changed. Open the logs for details."
            }
            Self::Unexpected => {
                "The application could not be loaded.\n\nYour saved profiles have not been changed. Open the logs for details."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "ApplicationLoadFailure::from_error" {
            case "Categorize Application Read Errors to User Failure Types" {
                given "Save Data, Storage, Device Key, Invariant Conditions, Unexpected Errors" {
                    let cases = [
                        (
                            ApplicationError::ProfileRepository(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "invalid profile file",
                            )),
                            ApplicationLoadFailure::InvalidStoredData,
                        ),
                        (
                            ApplicationError::ProfileRepository(std::io::Error::new(
                                std::io::ErrorKind::PermissionDenied,
                                "access denied",
                            )),
                            ApplicationLoadFailure::StorageUnavailable,
                        ),
                        (
                            ApplicationError::DeviceKey(Box::new(std::io::Error::other("TPM unavailable"))),
                            ApplicationLoadFailure::DeviceKeyUnavailable,
                        ),
                        (
                            ApplicationError::InvariantViolation("duplicate profile".into()),
                            ApplicationLoadFailure::InvariantViolation,
                        ),
                        (
                            ApplicationError::Certificate("unexpected during initialization".into()),
                            ApplicationLoadFailure::Unexpected,
                        ),
                        (
                            ApplicationError::PasskeyPinChangeRequired(Box::new(std::io::Error::other(
                                "PIN change required during initialization",
                            ))),
                            ApplicationLoadFailure::Unexpected,
                        ),
                    ];
                }
                when "Classify each Application Error into User Failure Types" {
                    let results =
                        cases.map(|(error, expected)| (ApplicationLoadFailure::from_error(&error), expected));
                }
                then "ERROR The failure type corresponding to the cause is selected" {
                    assert!(
                        results
                            .into_iter()
                            .all(|(actual, expected)| actual == expected)
                    );
                }
            }
        }
    }
}
