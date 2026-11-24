use keyringd_core::model::ProfileSshIdentityKind;

use keyringd_core::service::SshConnectionTestReport;

#[derive(Clone, Debug, PartialEq)]
pub enum SshConnectionTestEvent {
    HostChanged(String),
    IdentitySelected(ProfileSshIdentityKind),
    TestRequested,
    TestFinished(Result<SshConnectionTestReport, String>),
    ResultWindowOpened(Result<(), String>),
    CloseRequested,
    ResultCopyRequested,
    ResultClosed,
}
