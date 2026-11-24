use super::DeviceKeyProtectionResult;
use keyringd_core::application::ApplicationError;
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeviceKeyProtectionOutcome {
    Succeeded(DeviceKeyProtectionResult),
    Cancelled,
    Failed(String),
}

impl From<Result<DeviceKeyProtectionResult, ApplicationError>> for DeviceKeyProtectionOutcome {
    fn from(result: Result<DeviceKeyProtectionResult, ApplicationError>) -> Self {
        match result {
            Ok(result) => Self::Succeeded(result),
            Err(ApplicationError::OperationCanceled) => Self::Cancelled,
            Err(error) => Self::Failed(error.report_user_error(&error.user_message())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "DeviceKeyProtectionOutcome::from" {
            case "Preserves success and distinguishes cancellation from failure" {
                verify "represent success, user cancellation, and an error as distinct outcomes" {
                    let result = DeviceKeyProtectionResult {
                        notice: "Protection changed".into(),
                        registration_notice: None,
                    };
                    assert_eq!(
                        DeviceKeyProtectionOutcome::from(Ok(result.clone())),
                        DeviceKeyProtectionOutcome::Succeeded(result)
                    );
                    assert_eq!(
                        DeviceKeyProtectionOutcome::from(Err(ApplicationError::OperationCanceled)),
                        DeviceKeyProtectionOutcome::Cancelled
                    );
                    let error = ApplicationError::NotFound("profile");
                    let message = error.user_message();
                    assert!(
                        matches!(DeviceKeyProtectionOutcome::from(Err(error)), DeviceKeyProtectionOutcome::Failed(detail) if detail.contains(&message))
                    );
                }
            }
        }
    }
}
