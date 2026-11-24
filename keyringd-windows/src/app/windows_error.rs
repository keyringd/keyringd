#[derive(Debug, thiserror::Error)]
pub(crate) enum WindowsError {
    #[error("{0}")]
    Application(#[source] keyringd_core::application::ApplicationError),
    #[error("desktop environment failed: {0}")]
    Environment(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("operation canceled by the user")]
    OperationCanceled,
}

impl From<keyringd_core::application::ApplicationError> for WindowsError {
    fn from(error: keyringd_core::application::ApplicationError) -> Self {
        match error {
            keyringd_core::application::ApplicationError::OperationCanceled => {
                Self::OperationCanceled
            }
            error => Self::Application(error),
        }
    }
}

impl From<windows::core::Error> for WindowsError {
    fn from(error: windows::core::Error) -> Self {
        Self::Environment(error.into())
    }
}

impl WindowsError {
    pub(crate) fn report_user_error(&self, message: &str) -> String {
        match self {
            Self::Application(error) => error.report_user_error(message),
            Self::OperationCanceled => {
                keyringd_core::application::ApplicationError::OperationCanceled
                    .report_user_error(message)
            }
            Self::Environment(_) => keyringd_core::util::report_user_error(self, message),
        }
    }

    /// It does not display the OS diagnosis string on the screen, and returns the guidance according to the operation of the application.
    pub(crate) fn user_message(&self) -> String {
        match self {
            Self::Application(error) => error.user_message(),
            Self::Environment(_) => "Could not complete the desktop operation. Try again. If the problem continues, restart the app.".into(),
            Self::OperationCanceled => "The operation was canceled.".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    keyringd_testkit::scenario! {
        group "WindowsError::user_message" {
            case "Environment error retains io diagnostics without displaying them to the user" {
                verify "retain the I/O cause in diagnostics and show the concise environment error" {
                    let error = WindowsError::Environment(std::io::Error::from_raw_os_error(5).into());
                    let source = error
                        .source()
                        .unwrap()
                        .downcast_ref::<std::io::Error>()
                        .unwrap();
                    assert_eq!(source.raw_os_error(), Some(5));
                    assert!(!error.user_message().contains(&source.to_string()));
                }
            }
        }
    }
}
