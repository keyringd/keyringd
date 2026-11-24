#[derive(Debug)]
pub enum RemoveProfileEvent {
    CloseRequested,
    RemovalRequested,
    RemovalFinished(Result<(), String>),
}

impl RemoveProfileEvent {
    pub fn removal_finished(
        result: Result<(), keyringd_core::application::ApplicationError>,
    ) -> Self {
        Self::RemovalFinished(
            result.map_err(|error| error.report_user_error(&error.user_message())),
        )
    }

    pub fn notice(&self, name: &str) -> Option<crate::toast::ToastMessage> {
        use crate::toast::ToastMessage;
        match self {
            Self::RemovalFinished(Ok(())) => Some(ToastMessage::Success(
                crate::remove_profile_window::removed_notice(name),
            )),
            Self::RemovalFinished(Err(error)) => Some(ToastMessage::Error(
                crate::remove_profile_window::failed_notice(name, error),
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toast::ToastMessage;

    keyringd_testkit::scenario! {
        group "RemoveProfileEvent::notice" {
            case "Only completion produces a notice and failure keeps its detail" {
                verify "issue the completion notice on success and preserve the failure explanation" {
                    assert!(
                        RemoveProfileEvent::RemovalRequested
                            .notice("Work")
                            .is_none()
                    );
                    assert!(RemoveProfileEvent::CloseRequested.notice("Work").is_none());
                    assert!(
                        matches!(RemoveProfileEvent::removal_finished(Ok(())).notice("Work"), Some(ToastMessage::Success(message)) if message.contains("Work"))
                    );
                    let error = keyringd_core::application::ApplicationError::NotFound("profile");
                    let message = error.user_message();
                    assert!(
                        matches!(RemoveProfileEvent::removal_finished(Err(error)).notice("Work"), Some(ToastMessage::Error(detail)) if detail.contains("Work") && detail.contains(&message))
                    );
                }
            }
        }
    }
}
