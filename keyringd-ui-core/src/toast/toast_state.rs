use super::ToastMessage;
use std::time::Duration;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToastState {
    message: Option<ToastMessage>,
    revision: u64,
}

impl ToastState {
    pub fn show(&mut self, message: ToastMessage) -> u64 {
        self.revision += 1;
        self.message = Some(message);
        self.revision
    }

    pub fn message(&self) -> Option<&ToastMessage> {
        self.message.as_ref()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn auto_dismiss_after(&self) -> Option<Duration> {
        match self.message {
            Some(ToastMessage::Success(_)) => Some(Duration::from_secs(3)),
            _ => None,
        }
    }

    pub fn expire(&mut self, revision: u64) -> bool {
        self.auto_dismiss_after().is_some() && self.dismiss(revision)
    }

    pub fn dismiss(&mut self, revision: u64) -> bool {
        self.revision == revision && self.message.take().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "ToastState::show" {
            case "Repeating the same message restarts its display period" {
                verify "reset the dismissal timer when an identical toast is shown again" {
                    let mut state = ToastState::default();
                    let first = state.show(ToastMessage::Success("Copied".into()));
                    let second = state.show(ToastMessage::Success("Copied".into()));
                    assert_ne!(first, second);
                    assert_eq!(state.auto_dismiss_after(), Some(Duration::from_secs(3)));
                    assert!(!state.expire(first));
                    assert!(state.message().is_some());
                    assert!(state.expire(second));
                    assert!(state.message().is_none());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ToastState::dismiss" {
            case "Old timers and dismissals cannot clear a newer message" {
                verify "retain the current toast when its predecessor's timer or dismissal fires" {
                    let mut state = ToastState::default();
                    let old = state.show(ToastMessage::Success("Copied".into()));
                    let current = state.show(ToastMessage::Error("Copy failed".into()));
                    assert!(!state.expire(old));
                    assert!(!state.dismiss(old));
                    assert_eq!(
                        state.message(),
                        Some(&ToastMessage::Error("Copy failed".into()))
                    );
                    assert!(state.dismiss(current));
                    assert!(!state.dismiss(current));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ToastState::show" {
            case "Errors remain until dismissed or replaced" {
                verify "keep an error toast visible until the user dismisses it or another toast replaces it" {
                    let mut state = ToastState::default();
                    let error = state.show(ToastMessage::Error("Copy failed".into()));
                    assert_eq!(state.auto_dismiss_after(), None);
                    assert!(!state.expire(error));
                    let success = state.show(ToastMessage::Success("Copied".into()));
                    assert!(!state.dismiss(error));
                    assert!(state.expire(success));
                }
            }
        }
    }
}
