#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SshConnectionTestResult {
    Connected,
    SelectedIdentityUnconfirmed,
    AdditionalAuthenticationRequired,
    ClientConfigurationProblem(String),
    IdentityRejected,
    ConnectionFailed(String),
}
