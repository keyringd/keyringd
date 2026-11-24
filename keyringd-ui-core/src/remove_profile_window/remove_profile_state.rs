use super::{RemoveProfileCommand, RemoveProfileEvent};
use crate::ComponentModel;
use crate::text;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum RemoveProfileState {
    #[default]
    Confirming,
    Removing,
    Failed(String),
    Completed,
}

impl ComponentModel for RemoveProfileState {
    type Event = RemoveProfileEvent;
    type Command = RemoveProfileCommand;
    /// Hosts may close the view while removal is running. They must execute
    /// the Remove command to completion independently of the view's lifetime.
    fn update(&mut self, event: RemoveProfileEvent) -> Option<RemoveProfileCommand> {
        match event {
            RemoveProfileEvent::CloseRequested => Some(RemoveProfileCommand::Close),
            RemoveProfileEvent::RemovalRequested if self.can_remove() => {
                *self = Self::Removing;
                Some(RemoveProfileCommand::Remove)
            }
            RemoveProfileEvent::RemovalFinished(result) if self.is_removing() => match result {
                Ok(()) => {
                    *self = Self::Completed;
                    Some(RemoveProfileCommand::Close)
                }
                Err(error) => {
                    *self = Self::Failed(error);
                    None
                }
            },
            _ => None,
        }
    }
}

impl RemoveProfileState {
    pub fn can_remove(&self) -> bool {
        matches!(self, Self::Confirming | Self::Failed(_))
    }

    pub fn is_removing(&self) -> bool {
        matches!(self, Self::Removing)
    }

    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Failed(error) => Some(error),
            _ => None,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Confirming => text::REMOVE_PROFILE_DESCRIPTION,
            Self::Removing => text::REMOVE_PROFILE_REMOVING,
            Self::Failed(error) => error,
            Self::Completed => "",
        }
    }

    pub fn heading(&self, profile_name: &str) -> String {
        if matches!(self, Self::Failed(_)) {
            text::REMOVE_PROFILE_ERROR_TITLE.to_owned()
        } else {
            crate::remove_profile_window::heading(profile_name)
        }
    }

    pub fn remove_label(&self) -> &'static str {
        if matches!(self, Self::Failed(_)) {
            text::REMOVE_PROFILE_RETRY
        } else {
            text::REMOVE_PROFILE_REMOVE
        }
    }

    pub fn close_label(&self) -> &'static str {
        if self.is_removing() {
            crate::text::CLOSE
        } else {
            crate::text::CANCEL
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "RemoveProfileState::update" {
            case "Closing during removal keeps the operation running" {
                verify "leave the pending removal active when its window is closed" {
                    let mut state = RemoveProfileState::default();
                    state.update(RemoveProfileEvent::RemovalRequested);
                    assert_eq!(
                        state.update(RemoveProfileEvent::CloseRequested),
                        Some(RemoveProfileCommand::Close)
                    );
                    assert!(state.is_removing());
                    assert_eq!(state.update(RemoveProfileEvent::RemovalRequested), None);
                    assert_eq!(
                        state.update(RemoveProfileEvent::RemovalFinished(Ok(()))),
                        Some(RemoveProfileCommand::Close)
                    );
                    assert_eq!(state, RemoveProfileState::Completed);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "RemoveProfileState::update" {
            case "Starting removal immediately blocks duplicate requests" {
                verify "enter the removing state as soon as the first request is accepted" {
                    let mut state = RemoveProfileState::default();
                    assert_eq!(state.message(), text::REMOVE_PROFILE_DESCRIPTION);
                    assert_eq!(
                        state.update(RemoveProfileEvent::RemovalRequested),
                        Some(RemoveProfileCommand::Remove)
                    );
                    assert!(state.is_removing());
                    assert!(!state.can_remove());
                    assert_eq!(state.message(), text::REMOVE_PROFILE_REMOVING);
                    assert_eq!(state.update(RemoveProfileEvent::RemovalRequested), None);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "RemoveProfileState::update" {
            case "Failure keeps the dialog available for retry and retry clears the error" {
                verify "return to a retryable state on failure and clear the error when retry starts" {
                    let mut state = RemoveProfileState::default();
                    state.update(RemoveProfileEvent::RemovalRequested);
                    assert_eq!(
                        state.update(RemoveProfileEvent::RemovalFinished(Err(
                            "storage unavailable".into()
                        ))),
                        None
                    );
                    assert_eq!(state.error(), Some("storage unavailable"));
                    assert_eq!(state.message(), "storage unavailable");
                    assert_eq!(state.remove_label(), text::REMOVE_PROFILE_RETRY);
                    assert!(state.can_remove());
                    assert_eq!(
                        state.update(RemoveProfileEvent::RemovalRequested),
                        Some(RemoveProfileCommand::Remove)
                    );
                    assert_eq!(state.error(), None);
                    assert_eq!(state.message(), text::REMOVE_PROFILE_REMOVING);
                    assert_eq!(
                        state.update(RemoveProfileEvent::RemovalFinished(Ok(()))),
                        Some(RemoveProfileCommand::Close)
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "RemoveProfileState::update" {
            case "Successful removal closes once and cannot run again" {
                verify "emit a single close command after successful removal and ignore further requests" {
                    let mut state = RemoveProfileState::default();
                    state.update(RemoveProfileEvent::RemovalRequested);
                    assert_eq!(
                        state.update(RemoveProfileEvent::RemovalFinished(Ok(()))),
                        Some(RemoveProfileCommand::Close)
                    );
                    assert_eq!(state, RemoveProfileState::Completed);
                    assert!(!state.can_remove());
                    assert_eq!(state.update(RemoveProfileEvent::RemovalRequested), None);
                    assert_eq!(
                        state.update(RemoveProfileEvent::RemovalFinished(Ok(()))),
                        None
                    );
                    assert_eq!(
                        state.update(RemoveProfileEvent::RemovalFinished(Err(
                            "late failure".into()
                        ))),
                        None
                    );
                    assert_eq!(state, RemoveProfileState::Completed);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "RemoveProfileState::update" {
            case "Completion without an active removal does not close or overwrite the dialog" {
                verify "ignore a completion event received before any removal starts" {
                    for mut state in [
                        RemoveProfileState::Confirming,
                        RemoveProfileState::Failed("retry needed".into()),
                    ] {
                        let previous = state.clone();
                        assert_eq!(
                            state.update(RemoveProfileEvent::RemovalFinished(Ok(()))),
                            None
                        );
                        assert_eq!(
                            state.update(RemoveProfileEvent::RemovalFinished(Err(
                                "late failure".into()
                            ))),
                            None
                        );
                        assert_eq!(state, previous);
                    }
                }
            }
        }
    }
}
