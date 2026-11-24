use keyringd_core::model::ProfileSshIdentityKind;
use keyringd_core::service::SshConnectionTestReport;

#[derive(Clone, Debug, PartialEq)]
pub enum SshConnectionTestCommand {
    Close,
    CopyTrustEntry(String),
    Test {
        target: String,
        identity: ProfileSshIdentityKind,
    },
    OpenResult {
        target: String,
        report: SshConnectionTestReport,
    },
}
