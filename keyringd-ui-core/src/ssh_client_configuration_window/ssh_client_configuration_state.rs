use super::{
    SshClientConfigurationCommand, SshClientConfigurationCopyTarget, SshClientConfigurationEvent,
};
use crate::ComponentModel;
use crate::text;

#[derive(Default)]
pub struct SshClientConfigurationState {
    open: bool,
    configured: bool,
    phase: Phase,
    status: String,
    error: bool,
}

#[derive(Default)]
enum Phase {
    #[default]
    Ready,
    Confirming(Operation),
    Applying(Operation),
}

#[derive(Clone, Copy)]
enum Operation {
    Configuration,
    Copy(SshClientConfigurationCopyTarget),
}

impl SshClientConfigurationState {
    pub fn new(configured: Result<bool, String>) -> Self {
        match configured {
            Ok(configured) => Self {
                configured,
                ..Self::default()
            },
            Err(error) => Self {
                status: error,
                error: true,
                ..Self::default()
            },
        }
    }
    pub fn can_configure(&self) -> bool {
        !self.busy()
    }
    pub fn button_label(&self) -> &'static str {
        if matches!(self.phase, Phase::Applying(Operation::Configuration)) {
            text::SSH_CLIENT_CONFIGURATION_APPLYING
        } else if self.configured {
            text::SSH_CLIENT_CONFIGURATION_UNCONFIGURE
        } else {
            text::SSH_CLIENT_CONFIGURATION_CONFIGURE
        }
    }
    pub fn status_message(&self) -> Option<&str> {
        (!self.status.is_empty()).then_some(&self.status)
    }
    pub fn configured(&self) -> bool {
        self.configured
    }
    pub fn busy(&self) -> bool {
        !matches!(self.phase, Phase::Ready)
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn status(&self) -> (&str, bool) {
        (&self.status, self.error)
    }
    fn finish(&mut self, result: Result<(), String>, success: &'static str) {
        self.phase = Phase::Ready;
        match result {
            Ok(()) => {
                self.error = false;
                self.status = success.into();
            }
            Err(error) => {
                self.error = true;
                self.status = error;
            }
        }
    }
}
impl ComponentModel for SshClientConfigurationState {
    type Event = SshClientConfigurationEvent;
    type Command = SshClientConfigurationCommand;
    fn update(&mut self, event: Self::Event) -> Option<Self::Command> {
        match event {
            SshClientConfigurationEvent::ConfigurationChecked(result) if !self.busy() => {
                match result {
                    Ok(configured) => {
                        self.configured = configured;
                        self.status.clear();
                        self.error = false;
                    }
                    Err(error) => {
                        self.status = error;
                        self.error = true;
                    }
                }
                None
            }
            SshClientConfigurationEvent::OpenRequested => {
                self.open = true;
                None
            }
            SshClientConfigurationEvent::ConfigureRequested if !self.busy() => {
                self.phase = Phase::Confirming(Operation::Configuration);
                Some(SshClientConfigurationCommand::ConfirmConfiguration)
            }
            SshClientConfigurationEvent::CopyRequested(target) if !self.busy() => {
                self.phase = Phase::Confirming(Operation::Copy(target));
                Some(SshClientConfigurationCommand::ConfirmCopy(target))
            }
            SshClientConfigurationEvent::ConfirmationAccepted => {
                let Phase::Confirming(operation) = self.phase else {
                    return None;
                };
                self.phase = Phase::Applying(operation);
                self.error = false;
                Some(match operation {
                    Operation::Configuration => {
                        self.status = text::SSH_CLIENT_CONFIGURATION_APPLYING.into();
                        SshClientConfigurationCommand::Configure(!self.configured)
                    }
                    Operation::Copy(target) => {
                        self.status = text::SSH_CLIENT_CONFIGURATION_COPYING.into();
                        SshClientConfigurationCommand::Copy(target)
                    }
                })
            }
            SshClientConfigurationEvent::ConfigurationFinished(result)
                if matches!(self.phase, Phase::Applying(Operation::Configuration)) =>
            {
                if result.is_ok() {
                    self.configured = !self.configured;
                }
                self.finish(result, text::SSH_CLIENT_CONFIGURATION_APPLIED);
                None
            }
            SshClientConfigurationEvent::CopyFinished(result)
                if matches!(self.phase, Phase::Applying(Operation::Copy(_))) =>
            {
                self.finish(result, text::SSH_CLIENT_CONFIGURATION_COPIED);
                None
            }
            SshClientConfigurationEvent::ConfirmationCancelled
                if matches!(self.phase, Phase::Confirming(_)) =>
            {
                self.phase = Phase::Ready;
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refreshes_observed_configuration_without_overriding_an_active_operation() {
        let mut state = SshClientConfigurationState::new(Ok(false));
        state.update(SshClientConfigurationEvent::ConfigurationChecked(Ok(true)));
        assert!(state.configured());
        state.update(SshClientConfigurationEvent::ConfigureRequested);
        state.update(SshClientConfigurationEvent::ConfigurationChecked(Ok(false)));
        assert!(state.configured());
        assert!(matches!(
            state.update(SshClientConfigurationEvent::ConfirmationAccepted),
            Some(SshClientConfigurationCommand::Configure(false))
        ));
        state.update(SshClientConfigurationEvent::ConfigurationFinished(Ok(())));
        assert!(!state.configured());
        state.update(SshClientConfigurationEvent::ConfigurationChecked(Err(
            "query failed".into(),
        )));
        assert_eq!(state.status(), ("query failed", true));
    }

    keyringd_testkit::scenario! {
        group "SshClientConfigurationState::update" {
            case "Copy requires confirmation and does not change native configuration" {
                verify "copy the requested configuration only after confirmation and leave native state intact" {
                    for configured in [false, true] {
                        for target in [
                            SshClientConfigurationCopyTarget::Wsl,
                            SshClientConfigurationCopyTarget::GitForWindows,
                        ] {
                            let mut state = SshClientConfigurationState::new(Ok(configured));
                            assert!(
                                matches!(state.update(SshClientConfigurationEvent::CopyRequested(target)),
                                        Some(SshClientConfigurationCommand::ConfirmCopy(actual)) if actual == target)
                            );
                            assert!(!state.can_configure());
                            assert!(
                                state
                                    .update(SshClientConfigurationEvent::ConfigureRequested)
                                    .is_none()
                            );
                            assert!(
                                state
                                    .update(SshClientConfigurationEvent::CopyRequested(target))
                                    .is_none()
                            );
                            assert!(
                                matches!(state.update(SshClientConfigurationEvent::ConfirmationAccepted),
                                        Some(SshClientConfigurationCommand::Copy(actual)) if actual == target)
                            );
                            state.update(SshClientConfigurationEvent::ConfigurationFinished(Ok(())));
                            state.update(SshClientConfigurationEvent::ConfirmationCancelled);
                            assert!(state.busy());
                            assert!(
                                state
                                    .update(SshClientConfigurationEvent::ConfirmationAccepted)
                                    .is_none()
                            );
                            state.update(SshClientConfigurationEvent::CopyFinished(Ok(())));
                            assert_eq!(state.configured(), configured);
                            assert_eq!(
                                state.status(),
                                (text::SSH_CLIENT_CONFIGURATION_COPIED, false)
                            );
                            assert!(state.can_configure());
                            state.update(SshClientConfigurationEvent::CopyFinished(Err(
                                "late failure".into(),
                            )));
                            assert_eq!(
                                state.status(),
                                (text::SSH_CLIENT_CONFIGURATION_COPIED, false)
                            );
                        }
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshClientConfigurationState::update" {
            case "Copy cancellation and failure allow retry without changing configuration" {
                verify "allow the copy to be attempted again after cancellation or failure" {
                    let mut state = SshClientConfigurationState::new(Ok(true));
                    state.update(SshClientConfigurationEvent::CopyRequested(
                        SshClientConfigurationCopyTarget::Wsl,
                    ));
                    state.update(SshClientConfigurationEvent::ConfirmationCancelled);
                    assert!(!state.busy());
                    assert!(
                        state
                            .update(SshClientConfigurationEvent::ConfirmationAccepted)
                            .is_none()
                    );
                    assert_eq!(state.status(), ("", false));
                    state.update(SshClientConfigurationEvent::CopyRequested(
                        SshClientConfigurationCopyTarget::GitForWindows,
                    ));
                    state.update(SshClientConfigurationEvent::ConfirmationAccepted);
                    state.update(SshClientConfigurationEvent::CopyFinished(Err(
                        "clipboard unavailable".into(),
                    )));
                    assert!(state.configured());
                    assert_eq!(state.status(), ("clipboard unavailable", true));
                    state.update(SshClientConfigurationEvent::ConfigureRequested);
                    assert!(matches!(
                        state.update(SshClientConfigurationEvent::ConfirmationAccepted),
                        Some(SshClientConfigurationCommand::Configure(false))
                    ));
                    assert_eq!(
                        state.status(),
                        (text::SSH_CLIENT_CONFIGURATION_APPLYING, false)
                    );
                    state.update(SshClientConfigurationEvent::CopyFinished(Ok(())));
                    assert!(state.busy());
                    state.update(SshClientConfigurationEvent::ConfigurationFinished(Ok(())));
                    state.update(SshClientConfigurationEvent::ConfigurationFinished(Ok(())));
                    assert!(!state.configured());
                    assert_eq!(
                        state.status(),
                        (text::SSH_CLIENT_CONFIGURATION_APPLIED, false)
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshClientConfigurationState::update" {
            case "Unsolicited completions do not clear a query error" {
                verify "keep the configuration query failure when an unrelated completion arrives" {
                    let mut state = SshClientConfigurationState::new(Err("cannot read configuration".into()));
                    state.update(SshClientConfigurationEvent::ConfigurationFinished(Ok(())));
                    state.update(SshClientConfigurationEvent::CopyFinished(Ok(())));
                    assert_eq!(state.status(), ("cannot read configuration", true));
                    assert!(!state.busy());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshClientConfigurationState::update" {
            case "Cancellation and failure preserve the configuration" {
                verify "retain the last confirmed SSH client state after an unsuccessful change" {
                    let mut state = SshClientConfigurationState::new(Ok(true));
                    state.update(SshClientConfigurationEvent::ConfigureRequested);
                    state.update(SshClientConfigurationEvent::ConfirmationCancelled);
                    assert!(state.configured());
                    assert!(!state.busy());
                    state.update(SshClientConfigurationEvent::ConfigureRequested);
                    assert!(matches!(
                        state.update(SshClientConfigurationEvent::ConfirmationAccepted),
                        Some(SshClientConfigurationCommand::Configure(false))
                    ));
                    assert!(
                        state
                            .update(SshClientConfigurationEvent::ConfigureRequested)
                            .is_none()
                    );
                    state.update(SshClientConfigurationEvent::ConfigurationFinished(Err(
                        "failed".into(),
                    )));
                    assert!(state.configured());
                    assert!(!state.busy());
                    state.update(SshClientConfigurationEvent::ConfigureRequested);
                    state.update(SshClientConfigurationEvent::ConfirmationAccepted);
                    state.update(SshClientConfigurationEvent::ConfigurationFinished(Ok(())));
                    assert!(!state.configured());
                }
            }
        }
    }
}
