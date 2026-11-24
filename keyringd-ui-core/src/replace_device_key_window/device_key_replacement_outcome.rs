use keyringd_core::application::ApplicationError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeviceKeyReplacementOutcome {
    Completed,
    Cancelled,
    Failed(String),
}

impl From<Result<(), ApplicationError>> for DeviceKeyReplacementOutcome {
    fn from(result: Result<(), ApplicationError>) -> Self {
        match result {
            Ok(()) => Self::Completed,
            Err(ApplicationError::OperationCanceled) => Self::Cancelled,
            Err(error) => Self::Failed(error.report_user_error(&error.user_message())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "DeviceKeyReplacementOutcome::from" {
            case "Cancellation is neither completion nor failure" {
                verify "return to a retryable state after cancellation without showing a result" {
                    assert_eq!(
                        DeviceKeyReplacementOutcome::from(Ok(())),
                        DeviceKeyReplacementOutcome::Completed
                    );
                    assert_eq!(
                        DeviceKeyReplacementOutcome::from(Err(ApplicationError::OperationCanceled)),
                        DeviceKeyReplacementOutcome::Cancelled
                    );
                    let error = ApplicationError::NotFound("profile");
                    let message = error.user_message();
                    assert!(
                        matches!(DeviceKeyReplacementOutcome::from(Err(error)), DeviceKeyReplacementOutcome::Failed(detail) if detail.contains(&message))
                    );
                }
            }
        }
    }
}
