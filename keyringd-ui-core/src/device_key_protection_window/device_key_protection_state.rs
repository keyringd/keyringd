use super::{DeviceKeyProtectionCommand, DeviceKeyProtectionEvent, DeviceKeyProtectionOutcome};
use crate::ComponentModel;
use keyringd_core::service::DeviceKeyProtectionChange;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceKeyProtectionState {
    change: DeviceKeyProtectionChange,
    phase: Phase,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Phase {
    Ready,
    Authorizing,
    Completed,
    Failed(String),
    Cancelled,
}

impl DeviceKeyProtectionState {
    pub fn ready(change: DeviceKeyProtectionChange) -> Self {
        Self {
            change,
            phase: Phase::Ready,
        }
    }
    pub fn is_authorizing(&self) -> bool {
        self.phase == Phase::Authorizing
    }
    pub fn is_completed(&self) -> bool {
        self.phase == Phase::Completed
    }
    pub fn is_cancelled(&self) -> bool {
        self.phase == Phase::Cancelled
    }
    pub fn error(&self) -> Option<&str> {
        if let Phase::Failed(error) = &self.phase {
            Some(error)
        } else {
            None
        }
    }
}

impl ComponentModel for DeviceKeyProtectionState {
    type Event = DeviceKeyProtectionEvent;
    type Command = DeviceKeyProtectionCommand;

    fn update(&mut self, event: Self::Event) -> Option<Self::Command> {
        match event {
            DeviceKeyProtectionEvent::CloseRequested if !self.is_authorizing() => {
                Some(DeviceKeyProtectionCommand::Close)
            }
            DeviceKeyProtectionEvent::Started
                if matches!(
                    self.phase,
                    Phase::Ready | Phase::Failed(_) | Phase::Cancelled
                ) =>
            {
                self.phase = Phase::Authorizing;
                Some(DeviceKeyProtectionCommand::Change(self.change))
            }
            DeviceKeyProtectionEvent::Finished(outcome) if self.is_authorizing() => match outcome {
                DeviceKeyProtectionOutcome::Succeeded(result) => {
                    self.phase = Phase::Completed;
                    Some(DeviceKeyProtectionCommand::Notify(result))
                }
                DeviceKeyProtectionOutcome::Cancelled => {
                    self.phase = Phase::Cancelled;
                    Some(DeviceKeyProtectionCommand::Close)
                }
                DeviceKeyProtectionOutcome::Failed(error) => {
                    self.phase = Phase::Failed(error.clone());
                    Some(DeviceKeyProtectionCommand::ShowRetry(error))
                }
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_key_protection_window::DeviceKeyProtectionResult;
    fn success() -> DeviceKeyProtectionOutcome {
        DeviceKeyProtectionOutcome::Succeeded(DeviceKeyProtectionResult {
            notice: "changed".into(),
            registration_notice: None,
        })
    }

    keyringd_testkit::scenario! {
        group "DeviceKeyProtectionState::update" {
            case "Closing is blocked only while authorizing" {
                verify "prevent closing during authorization and allow it again afterward" {
                    let mut state = DeviceKeyProtectionState::ready(DeviceKeyProtectionChange::Enable);
                    assert_eq!(
                        state.update(DeviceKeyProtectionEvent::CloseRequested),
                        Some(DeviceKeyProtectionCommand::Close)
                    );
                    state.update(DeviceKeyProtectionEvent::Started);
                    assert_eq!(state.update(DeviceKeyProtectionEvent::CloseRequested), None);
                    assert!(state.is_authorizing());
                    state.update(DeviceKeyProtectionEvent::Finished(
                        DeviceKeyProtectionOutcome::Cancelled,
                    ));
                    assert_eq!(
                        state.update(DeviceKeyProtectionEvent::CloseRequested),
                        Some(DeviceKeyProtectionCommand::Close)
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "DeviceKeyProtectionState::update" {
            case "Every change starts once and ignores unsolicited completion" {
                verify "dispatch each requested change once and drop premature results" {
                    for change in [
                        DeviceKeyProtectionChange::Enable,
                        DeviceKeyProtectionChange::ChangePassword,
                        DeviceKeyProtectionChange::Disable,
                    ] {
                        let mut state = DeviceKeyProtectionState::ready(change);
                        assert_eq!(
                            state.update(DeviceKeyProtectionEvent::Finished(success())),
                            None
                        );
                        assert_eq!(
                            state.update(DeviceKeyProtectionEvent::Started),
                            Some(DeviceKeyProtectionCommand::Change(change))
                        );
                        assert!(state.is_authorizing());
                        assert_eq!(state.update(DeviceKeyProtectionEvent::Started), None);
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "DeviceKeyProtectionState::update" {
            case "Cancellation has no success notice and can be retried" {
                verify "leave the success notice absent after cancellation and permit another request" {
                    let mut state = DeviceKeyProtectionState::ready(DeviceKeyProtectionChange::Enable);
                    state.update(DeviceKeyProtectionEvent::Started);
                    assert_eq!(
                        state.update(DeviceKeyProtectionEvent::Finished(
                            DeviceKeyProtectionOutcome::Cancelled
                        )),
                        Some(DeviceKeyProtectionCommand::Close)
                    );
                    assert!(state.is_cancelled());
                    assert!(!state.is_completed());
                    assert_eq!(
                        state.update(DeviceKeyProtectionEvent::Started),
                        Some(DeviceKeyProtectionCommand::Change(
                            DeviceKeyProtectionChange::Enable
                        ))
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "DeviceKeyProtectionState::update" {
            case "Failure offers retry once and retry retains the requested change" {
                verify "clear the failure when retry dispatches the same protection change" {
                    let mut state = DeviceKeyProtectionState::ready(DeviceKeyProtectionChange::ChangePassword);
                    state.update(DeviceKeyProtectionEvent::Started);
                    assert_eq!(
                        state.update(DeviceKeyProtectionEvent::Finished(
                            DeviceKeyProtectionOutcome::Failed("failed".into())
                        )),
                        Some(DeviceKeyProtectionCommand::ShowRetry("failed".into()))
                    );
                    assert_eq!(state.error(), Some("failed"));
                    assert_eq!(
                        state.update(DeviceKeyProtectionEvent::Finished(success())),
                        None
                    );
                    assert_eq!(
                        state.update(DeviceKeyProtectionEvent::Started),
                        Some(DeviceKeyProtectionCommand::Change(
                            DeviceKeyProtectionChange::ChangePassword
                        ))
                    );
                    assert_eq!(state.error(), None);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "DeviceKeyProtectionState::update" {
            case "Cancelling after failure closes without a success notification" {
                verify "close from the failure state without emitting a success notice" {
                    let mut state = DeviceKeyProtectionState::ready(DeviceKeyProtectionChange::Enable);
                    state.update(DeviceKeyProtectionEvent::Started);
                    assert_eq!(
                        state.update(DeviceKeyProtectionEvent::Finished(
                            DeviceKeyProtectionOutcome::Failed("failed".into())
                        )),
                        Some(DeviceKeyProtectionCommand::ShowRetry("failed".into()))
                    );
                    assert_eq!(
                        state.update(DeviceKeyProtectionEvent::CloseRequested),
                        Some(DeviceKeyProtectionCommand::Close)
                    );
                    assert!(!state.is_completed());
                    assert_eq!(
                        state.update(DeviceKeyProtectionEvent::Finished(success())),
                        None
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "DeviceKeyProtectionState::update" {
            case "Success is terminal and notifies once" {
                verify "deliver the success notification once and ignore later events" {
                    let mut state = DeviceKeyProtectionState::ready(DeviceKeyProtectionChange::Disable);
                    state.update(DeviceKeyProtectionEvent::Started);
                    assert!(matches!(
                        state.update(DeviceKeyProtectionEvent::Finished(success())),
                        Some(DeviceKeyProtectionCommand::Notify(_))
                    ));
                    assert!(state.is_completed());
                    assert_eq!(state.update(DeviceKeyProtectionEvent::Started), None);
                    assert_eq!(
                        state.update(DeviceKeyProtectionEvent::Finished(success())),
                        None
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "DeviceKeyProtectionState::update" {
            case "Completion delivers notice and registration notice together once" {
                verify "send both completion notices in one successful result" {
                    use keyringd_core::model::{DeviceKeyRegistrationNotice, ProfileId};

                    for registration_notice in [
                        None,
                        Some(DeviceKeyRegistrationNotice {
                            profile_id: ProfileId::from_stored("work".into()),
                            message: "Replace the server registration".into(),
                        }),
                    ] {
                        let mut state = DeviceKeyProtectionState::ready(DeviceKeyProtectionChange::Enable);
                        state.update(DeviceKeyProtectionEvent::Started);
                        let result = DeviceKeyProtectionResult {
                            notice: "Protection enabled".into(),
                            registration_notice,
                        };
                        let completion = || {
                            DeviceKeyProtectionEvent::Finished(DeviceKeyProtectionOutcome::Succeeded(
                                result.clone(),
                            ))
                        };
                        assert_eq!(
                            state.update(completion()),
                            Some(DeviceKeyProtectionCommand::Notify(result.clone()))
                        );
                        assert!(state.update(completion()).is_none());
                    }
                }
            }
        }
    }
}
