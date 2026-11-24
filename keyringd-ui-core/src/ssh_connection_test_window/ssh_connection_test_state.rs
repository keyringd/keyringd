use keyringd_core::model::ProfileSshIdentityKind;
use keyringd_core::service::{SshConnectionTestReport, SshConnectionTestResult};

use super::{SshConnectionTestCommand, SshConnectionTestEvent};
use crate::ComponentModel;

#[derive(Clone, Debug, PartialEq)]
pub struct SshConnectionTestState {
    host: String,
    identities: Vec<ProfileSshIdentityKind>,
    identity: ProfileSshIdentityKind,
    phase: Phase,
}

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    Ready { error: Option<String> },
    Testing,
    ShowingResult(SshConnectionTestReport),
}

impl SshConnectionTestState {
    pub fn new(identities: Vec<ProfileSshIdentityKind>) -> Self {
        let identity = *identities
            .first()
            .expect("SSH test requires an available identity");
        Self {
            host: String::new(),
            identities,
            identity,
            phase: Phase::Ready { error: None },
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }
    pub fn identities(&self) -> &[ProfileSshIdentityKind] {
        &self.identities
    }
    pub fn identity(&self) -> ProfileSshIdentityKind {
        self.identity
    }
    pub fn can_edit(&self) -> bool {
        matches!(self.phase, Phase::Ready { .. })
    }
    pub fn can_test(&self) -> bool {
        self.can_edit() && valid_host(self.host.trim())
    }
    pub fn is_testing(&self) -> bool {
        matches!(self.phase, Phase::Testing)
    }
    pub fn error(&self) -> Option<&str> {
        match &self.phase {
            Phase::Ready { error } => error.as_deref(),
            _ => None,
        }
    }
}

impl ComponentModel for SshConnectionTestState {
    type Event = SshConnectionTestEvent;
    type Command = SshConnectionTestCommand;

    fn update(&mut self, event: Self::Event) -> Option<Self::Command> {
        match event {
            SshConnectionTestEvent::CloseRequested if !self.is_testing() => {
                return Some(SshConnectionTestCommand::Close);
            }
            SshConnectionTestEvent::HostChanged(host) if self.can_edit() => self.host = host,
            SshConnectionTestEvent::IdentitySelected(identity)
                if self.can_edit() && self.identities.contains(&identity) =>
            {
                self.identity = identity;
            }
            SshConnectionTestEvent::TestRequested if self.can_test() => {
                self.host = self.host.trim().to_owned();
                self.phase = Phase::Testing;
                return Some(SshConnectionTestCommand::Test {
                    target: self.host.clone(),
                    identity: self.identity,
                });
            }
            SshConnectionTestEvent::TestFinished(result) if self.is_testing() => match result {
                Ok(result) => {
                    self.phase = Phase::ShowingResult(result.clone());
                    return Some(SshConnectionTestCommand::OpenResult {
                        target: self.host.clone(),
                        report: result,
                    });
                }
                Err(error) => self.phase = Phase::Ready { error: Some(error) },
            },
            SshConnectionTestEvent::ResultWindowOpened(Err(error))
                if matches!(self.phase, Phase::ShowingResult(_)) =>
            {
                self.phase = Phase::Ready { error: Some(error) };
            }
            SshConnectionTestEvent::ResultClosed
                if matches!(self.phase, Phase::ShowingResult(_)) =>
            {
                self.phase = Phase::Ready { error: None };
            }
            SshConnectionTestEvent::ResultCopyRequested => {
                if let Phase::ShowingResult(report) = &self.phase
                    && report.result == SshConnectionTestResult::IdentityRejected
                {
                    return Some(SshConnectionTestCommand::CopyTrustEntry(
                        report.trust_entry.clone(),
                    ));
                }
            }
            _ => {}
        }
        None
    }
}

fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && !host.starts_with('-')
        && !host.chars().any(|c| c.is_whitespace() || c.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(result: SshConnectionTestResult) -> SshConnectionTestReport {
        SshConnectionTestReport {
            result,
            identity: ProfileSshIdentityKind::DeviceKey,
            trust_entry: "tested key".into(),
            public_key_algorithm: "ecdsa-sha2-nistp256".into(),
            openssh_algorithm_directives: vec![],
        }
    }

    fn ready() -> SshConnectionTestState {
        SshConnectionTestState::new(vec![
            ProfileSshIdentityKind::DeviceKey,
            ProfileSshIdentityKind::Passkey,
        ])
    }

    fn testing() -> SshConnectionTestState {
        let mut state = ready();
        state.update(SshConnectionTestEvent::HostChanged("host".into()));
        state.update(SshConnectionTestEvent::TestRequested);
        state
    }

    keyringd_testkit::scenario! {
        group "SshConnectionTestState::update" {
            case "Close is blocked while a test is running" {
                verify "prevent closing the window until the connection test returns" {
                    let mut state = ready();
                    assert_eq!(
                        state.update(SshConnectionTestEvent::CloseRequested),
                        Some(SshConnectionTestCommand::Close)
                    );
                    state = testing();
                    assert_eq!(state.update(SshConnectionTestEvent::CloseRequested), None);
                    assert!(state.is_testing());
                    state.update(SshConnectionTestEvent::TestFinished(Ok(report(
                        SshConnectionTestResult::Connected,
                    ))));
                    assert_eq!(
                        state.update(SshConnectionTestEvent::CloseRequested),
                        Some(SshConnectionTestCommand::Close)
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshConnectionTestState::update" {
            case "Only available identities can be selected" {
                verify "reject selection of an unavailable SSH identity" {
                    let mut state = ready();
                    state.update(SshConnectionTestEvent::IdentitySelected(
                        ProfileSshIdentityKind::DeviceCertificate,
                    ));
                    assert_eq!(state.identity(), ProfileSshIdentityKind::DeviceKey);
                    state.update(SshConnectionTestEvent::IdentitySelected(
                        ProfileSshIdentityKind::Passkey,
                    ));
                    assert_eq!(state.identity(), ProfileSshIdentityKind::Passkey);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshConnectionTestState::update" {
            case "Only valid destinations start a test" {
                verify "emit a connection test command only for a supported destination" {
                    let mut state = ready();
                    for host in ["", " ", "-oProxyCommand=x", "host extra", "host\nextra"] {
                        state.update(SshConnectionTestEvent::HostChanged(host.into()));
                        assert!(!state.can_test());
                        assert_eq!(state.update(SshConnectionTestEvent::TestRequested), None);
                    }
                    state.update(SshConnectionTestEvent::HostChanged(" user@host ".into()));
                    state.update(SshConnectionTestEvent::IdentitySelected(
                        ProfileSshIdentityKind::Passkey,
                    ));
                    assert_eq!(
                        state.update(SshConnectionTestEvent::TestRequested),
                        Some(SshConnectionTestCommand::Test {
                            target: "user@host".into(),
                            identity: ProfileSshIdentityKind::Passkey,
                        })
                    );
                    state.update(SshConnectionTestEvent::HostChanged("other".into()));
                    state.update(SshConnectionTestEvent::IdentitySelected(
                        ProfileSshIdentityKind::DeviceKey,
                    ));
                    assert_eq!(state.host(), "user@host");
                    assert_eq!(state.identity(), ProfileSshIdentityKind::Passkey);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshConnectionTestState::update" {
            case "Duplicate requests and completions do not repeat native commands" {
                verify "emit at most one native start and result presentation for a request" {
                    let mut state = testing();
                    assert_eq!(state.update(SshConnectionTestEvent::TestRequested), None);
                    assert_eq!(
                        state.update(SshConnectionTestEvent::TestFinished(Ok(report(
                            SshConnectionTestResult::Connected
                        )))),
                        Some(SshConnectionTestCommand::OpenResult {
                            target: "host".into(),
                            report: report(SshConnectionTestResult::Connected),
                        })
                    );
                    assert!(!state.can_edit());
                    assert_eq!(
                        state.update(SshConnectionTestEvent::TestFinished(Ok(report(
                            SshConnectionTestResult::Connected
                        )))),
                        None
                    );
                    assert_eq!(state.update(SshConnectionTestEvent::TestRequested), None);
                    state.update(SshConnectionTestEvent::ResultWindowOpened(Ok(())));
                    assert!(!state.can_test());
                    state.update(SshConnectionTestEvent::ResultClosed);
                    assert!(state.can_test());
                    assert_eq!(state.host(), "host");
                    assert_eq!(state.identity(), ProfileSshIdentityKind::DeviceKey);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshConnectionTestState::update" {
            case "Copy uses the rejected identity and ignores stale requests" {
                verify "use the tested identity from the rejection and ignore a stale copy event" {
                    let mut state = testing();
                    assert_eq!(
                        state.update(SshConnectionTestEvent::ResultCopyRequested),
                        None
                    );
                    state.update(SshConnectionTestEvent::TestFinished(Ok(report(
                        SshConnectionTestResult::IdentityRejected,
                    ))));
                    assert_eq!(
                        state.update(SshConnectionTestEvent::ResultCopyRequested),
                        Some(SshConnectionTestCommand::CopyTrustEntry(
                            "tested key".into()
                        ))
                    );
                    state.update(SshConnectionTestEvent::ResultClosed);
                    assert_eq!(
                        state.update(SshConnectionTestEvent::ResultCopyRequested),
                        None
                    );
                    state.update(SshConnectionTestEvent::TestRequested);
                    state.update(SshConnectionTestEvent::TestFinished(Ok(report(
                        SshConnectionTestResult::Connected,
                    ))));
                    assert_eq!(
                        state.update(SshConnectionTestEvent::ResultCopyRequested),
                        None
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshConnectionTestState::update" {
            case "Execution and window open failures allow retry and clear errors on retry" {
                verify "retry the test or result window after clearing its last error" {
                    let mut state = testing();
                    state.update(SshConnectionTestEvent::TestFinished(Err(
                        "execution failed".into(),
                    )));
                    assert_eq!(state.error(), Some("execution failed"));
                    assert!(state.can_test());
                    state.update(SshConnectionTestEvent::TestRequested);
                    assert_eq!(state.error(), None);
                    state.update(SshConnectionTestEvent::TestFinished(Ok(report(
                        SshConnectionTestResult::Connected,
                    ))));
                    state.update(SshConnectionTestEvent::ResultWindowOpened(Err(
                        "window failed".into(),
                    )));
                    assert_eq!(state.error(), Some("window failed"));
                    assert!(state.can_test());
                    state.update(SshConnectionTestEvent::TestRequested);
                    state.update(SshConnectionTestEvent::ResultClosed);
                    state.update(SshConnectionTestEvent::ResultWindowOpened(Err(
                        "stale window".into(),
                    )));
                    assert!(state.is_testing());
                    assert_eq!(state.error(), None);
                }
            }
        }
    }
}
