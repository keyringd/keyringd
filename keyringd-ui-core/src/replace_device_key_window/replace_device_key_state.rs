use super::{DeviceKeyReplacementOutcome, ReplaceDeviceKeyCommand, ReplaceDeviceKeyEvent};
use crate::ComponentModel;
use crate::text;

/// Product state shared by native views.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ReplaceDeviceKeyState {
    #[default]
    Confirming,
    Authorizing,
    Completed,
    Failed(String),
    Cancelled,
    Closed,
}

impl ComponentModel for ReplaceDeviceKeyState {
    type Event = ReplaceDeviceKeyEvent;
    type Command = ReplaceDeviceKeyCommand;
    fn update(&mut self, event: ReplaceDeviceKeyEvent) -> Option<ReplaceDeviceKeyCommand> {
        match event {
            ReplaceDeviceKeyEvent::PrimaryRequested if self.is_complete() => {
                *self = Self::Closed;
                Some(ReplaceDeviceKeyCommand::OpenTrustEntriesAndClose)
            }
            ReplaceDeviceKeyEvent::PrimaryRequested
                if !self.is_busy() && !matches!(self, Self::Closed) =>
            {
                *self = Self::Authorizing;
                Some(ReplaceDeviceKeyCommand::Replace)
            }
            ReplaceDeviceKeyEvent::CloseRequested
                if !self.is_busy() && !matches!(self, Self::Closed) =>
            {
                *self = Self::Closed;
                Some(ReplaceDeviceKeyCommand::Close)
            }
            ReplaceDeviceKeyEvent::ReplacementFinished(result) if self.is_busy() => {
                *self = match result {
                    DeviceKeyReplacementOutcome::Completed => Self::Completed,
                    DeviceKeyReplacementOutcome::Cancelled => Self::Cancelled,
                    DeviceKeyReplacementOutcome::Failed(error) => Self::Failed(error),
                };
                None
            }
            _ => None,
        }
    }
}

impl ReplaceDeviceKeyState {
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Authorizing)
    }
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Completed)
    }
    pub fn error(&self) -> Option<&str> {
        if let Self::Failed(error) = self {
            Some(error)
        } else {
            None
        }
    }
    pub fn heading(&self) -> &str {
        match self {
            Self::Completed => text::REPLACE_DEVICE_KEY_COMPLETED_TITLE,
            Self::Failed(_) => text::REPLACE_DEVICE_KEY_ERROR_TITLE,
            _ => text::REPLACE_DEVICE_KEY_CONFIRM_TITLE,
        }
    }
    pub fn primary_label(&self) -> &str {
        match self {
            Self::Completed => text::TRUST_ENTRIES_MENU_ACTION,
            Self::Failed(_) | Self::Cancelled => text::REPLACE_DEVICE_KEY_RETRY,
            _ => text::REPLACE_DEVICE_KEY_REPLACE,
        }
    }
    pub fn close_label(&self) -> &str {
        if self.is_complete() {
            crate::text::CLOSE
        } else {
            crate::text::CANCEL
        }
    }
    pub fn message(&self) -> &str {
        match self {
            Self::Confirming => text::REPLACE_DEVICE_KEY_DESCRIPTION,
            Self::Authorizing => text::REPLACE_DEVICE_KEY_AUTHORIZING,
            Self::Completed => text::REPLACE_DEVICE_KEY_COMPLETED,
            Self::Failed(error) => error,
            Self::Cancelled => text::REPLACE_DEVICE_KEY_CANCELLED,
            Self::Closed => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "ReplaceDeviceKeyState::update" {
            case "Starting synchronously blocks duplicate requests and close" {
                verify "mark the replacement pending before dispatch to block a second start or close" {
                    let mut state = ReplaceDeviceKeyState::default();
                    assert_eq!(
                        state.update(ReplaceDeviceKeyEvent::PrimaryRequested),
                        Some(ReplaceDeviceKeyCommand::Replace)
                    );
                    assert!(state.is_busy());
                    assert_eq!(state.update(ReplaceDeviceKeyEvent::PrimaryRequested), None);
                    assert_eq!(state.update(ReplaceDeviceKeyEvent::CloseRequested), None);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ReplaceDeviceKeyState::update" {
            case "Failure and authorization cancellation allow retry" {
                verify "retain the ability to retry after either a failed or cancelled replacement" {
                    let mut state = ReplaceDeviceKeyState::default();
                    state.update(ReplaceDeviceKeyEvent::PrimaryRequested);
                    state.update(ReplaceDeviceKeyEvent::ReplacementFinished(
                        DeviceKeyReplacementOutcome::Failed("failed".into()),
                    ));
                    assert_eq!(state.error(), Some("failed"));
                    assert_eq!(state.primary_label(), text::REPLACE_DEVICE_KEY_RETRY);
                    assert_eq!(
                        state.update(ReplaceDeviceKeyEvent::PrimaryRequested),
                        Some(ReplaceDeviceKeyCommand::Replace)
                    );
                    assert_eq!(state.error(), None);
                    state.update(ReplaceDeviceKeyEvent::ReplacementFinished(
                        DeviceKeyReplacementOutcome::Cancelled,
                    ));
                    assert_eq!(state.message(), text::REPLACE_DEVICE_KEY_CANCELLED);
                    assert_eq!(
                        state.update(ReplaceDeviceKeyEvent::PrimaryRequested),
                        Some(ReplaceDeviceKeyCommand::Replace)
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ReplaceDeviceKeyState::update" {
            case "Opening public keys finishes the completed window once" {
                verify "open the public-key window once from the successful result" {
                    let mut state = ReplaceDeviceKeyState::default();
                    state.update(ReplaceDeviceKeyEvent::PrimaryRequested);
                    state.update(ReplaceDeviceKeyEvent::ReplacementFinished(
                        DeviceKeyReplacementOutcome::Completed,
                    ));
                    assert_eq!(
                        state.update(ReplaceDeviceKeyEvent::PrimaryRequested),
                        Some(ReplaceDeviceKeyCommand::OpenTrustEntriesAndClose)
                    );
                    assert_eq!(state.update(ReplaceDeviceKeyEvent::PrimaryRequested), None);
                    assert_eq!(state.update(ReplaceDeviceKeyEvent::CloseRequested), None);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ReplaceDeviceKeyState::update" {
            case "Unsolicited and duplicate completions are ignored" {
                verify "apply only the first completion for an active replacement" {
                    let mut state = ReplaceDeviceKeyState::default();
                    state.update(ReplaceDeviceKeyEvent::ReplacementFinished(
                        DeviceKeyReplacementOutcome::Completed,
                    ));
                    assert!(!state.is_complete());
                    state.update(ReplaceDeviceKeyEvent::PrimaryRequested);
                    state.update(ReplaceDeviceKeyEvent::ReplacementFinished(
                        DeviceKeyReplacementOutcome::Completed,
                    ));
                    state.update(ReplaceDeviceKeyEvent::ReplacementFinished(
                        DeviceKeyReplacementOutcome::Failed("late".into()),
                    ));
                    assert!(state.is_complete());
                }
            }
        }
    }
}
