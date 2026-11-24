use super::{AppMenuCommand, AppMenuEvent, AppMenuLifecycleEvent};
use crate::ComponentModel;

#[derive(Default)]
pub struct AppMenuState {
    phase: Phase,
}

#[derive(Default)]
enum Phase {
    #[default]
    Initial,
    Running,
    Stopped,
}

impl AppMenuState {
    pub fn is_stopped(&self) -> bool {
        matches!(self.phase, Phase::Stopped)
    }
}

impl ComponentModel for AppMenuState {
    type Event = AppMenuLifecycleEvent;
    type Command = AppMenuCommand;

    fn update(&mut self, event: Self::Event) -> Option<Self::Command> {
        match event {
            AppMenuLifecycleEvent::Mounted(profiles) if matches!(self.phase, Phase::Initial) => {
                self.phase = Phase::Running;
                Some(AppMenuCommand::RebuildMenu(profiles))
            }
            AppMenuLifecycleEvent::ProfilesChanged(profiles)
                if matches!(self.phase, Phase::Running) =>
            {
                Some(AppMenuCommand::RebuildMenu(profiles))
            }
            AppMenuLifecycleEvent::ActionRequested(action)
                if matches!(self.phase, Phase::Running) =>
            {
                if matches!(action, AppMenuEvent::Quit) {
                    self.phase = Phase::Stopped;
                    Some(AppMenuCommand::Stop {
                        action: Some(action),
                    })
                } else {
                    Some(AppMenuCommand::Emit(action))
                }
            }
            AppMenuLifecycleEvent::StopRequested if !self.is_stopped() => {
                self.phase = Phase::Stopped;
                Some(AppMenuCommand::Stop { action: None })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_core::model::ProfileId;

    fn assert_stopped(state: &mut AppMenuState) {
        assert!(state.is_stopped());
        for event in [
            AppMenuLifecycleEvent::Mounted(Vec::new()),
            AppMenuLifecycleEvent::ProfilesChanged(Vec::new()),
            AppMenuLifecycleEvent::ActionRequested(AppMenuEvent::ShowAbout),
            AppMenuLifecycleEvent::ActionRequested(AppMenuEvent::Quit),
            AppMenuLifecycleEvent::StopRequested,
        ] {
            assert!(state.update(event).is_none());
        }
    }

    keyringd_testkit::scenario! {
        group "AppMenuState::update" {
            case "Mount is required and only initializes the menu once" {
                verify "build the menu on the first mount and ignore repeated mounts" {
                    let mut state = AppMenuState::default();
                    assert!(
                        state
                            .update(AppMenuLifecycleEvent::ProfilesChanged(Vec::new()))
                            .is_none()
                    );
                    assert!(
                        state
                            .update(AppMenuLifecycleEvent::ActionRequested(
                                AppMenuEvent::ShowAbout
                            ))
                            .is_none()
                    );
                    assert!(matches!(
                        state.update(AppMenuLifecycleEvent::Mounted(Vec::new())),
                        Some(AppMenuCommand::RebuildMenu(_))
                    ));
                    assert!(
                        state
                            .update(AppMenuLifecycleEvent::Mounted(Vec::new()))
                            .is_none()
                    );
                    assert!(matches!(
                        state.update(AppMenuLifecycleEvent::ProfilesChanged(Vec::new())),
                        Some(AppMenuCommand::RebuildMenu(_))
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AppMenuState::update" {
            case "Running menu forwards the selected profile and enabled value" {
                verify "forward the selected profile ID and its requested enabled state" {
                    let mut state = AppMenuState::default();
                    state.update(AppMenuLifecycleEvent::Mounted(Vec::new()));
                    let id = ProfileId::from_stored("work".into());
                    assert!(
                        matches!(state.update(AppMenuLifecycleEvent::ActionRequested(
                                AppMenuEvent::SetProfileEnabled { id: id.clone(), enabled: false }
                            )), Some(AppMenuCommand::Emit(AppMenuEvent::SetProfileEnabled { id: actual, enabled: false })) if actual == id)
                    );
                    assert!(!state.is_stopped());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AppMenuState::update" {
            case "Explicit stop is terminal even before mount" {
                verify "suppress menu creation after a stop event received before mounting" {
                    for mounted in [false, true] {
                        let mut state = AppMenuState::default();
                        if mounted {
                            state.update(AppMenuLifecycleEvent::Mounted(Vec::new()));
                        }
                        assert!(matches!(
                            state.update(AppMenuLifecycleEvent::StopRequested),
                            Some(AppMenuCommand::Stop { action: None })
                        ));
                        assert_stopped(&mut state);
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AppMenuState::update" {
            case "Quit removes the menu and emits quit once before rejecting late events" {
                verify "emit one quit command, remove the menu, and ignore late events" {
                    let mut state = AppMenuState::default();
                    state.update(AppMenuLifecycleEvent::Mounted(Vec::new()));
                    assert!(matches!(
                        state.update(AppMenuLifecycleEvent::ActionRequested(AppMenuEvent::Quit)),
                        Some(AppMenuCommand::Stop {
                            action: Some(AppMenuEvent::Quit)
                        })
                    ));
                    assert_stopped(&mut state);
                }
            }
        }
    }
}
