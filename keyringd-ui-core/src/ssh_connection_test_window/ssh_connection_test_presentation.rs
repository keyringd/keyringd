use keyringd_core::service::{SshConnectionTestReport, SshConnectionTestResult};

pub struct SshConnectionTestPresentation {
    pub heading: String,
    pub body: String,
    /// The exact authorized_keys entry and server configuration guidance.
    pub troubleshooting: Option<(String, String)>,
}

impl SshConnectionTestPresentation {
    pub fn body_with_host(&self, target: &str) -> String {
        format!(
            "{}\n\n{}",
            crate::ssh_connection_test_window::host_label(target),
            self.body
        )
    }

    pub fn new(report: &SshConnectionTestReport) -> Self {
        let identity_name = report.identity.display_name();
        let (heading, body, troubleshooting) = match &report.result {
            SshConnectionTestResult::Connected => (
                "SSH Connection Succeeded".to_owned(),
                format!("OpenSSH authenticated with this profile’s {identity_name}."),
                None,
            ),
            SshConnectionTestResult::SelectedIdentityUnconfirmed => (
                format!("{identity_name} Not Verified"),
                "SSH authentication was successful, but the authentication with the selected key could not be verified".to_owned(),
                None,
            ),
            SshConnectionTestResult::AdditionalAuthenticationRequired => (
                format!("{identity_name} Accepted"),
                "The key has been accepted, but additional authentication is required".to_owned(),
                None,
            ),
            SshConnectionTestResult::ClientConfigurationProblem(problem) => (
                "SSH Configuration Needs Attention".to_owned(),
                problem.clone(),
                None,
            ),
            SshConnectionTestResult::IdentityRejected => (
                format!("{identity_name} Rejected"),
                format!(
                    "OpenSSH offered this profile’s {identity_name}, but the server rejected it."
                ),
                Some((
                    report.trust_entry.clone(),
                    format!(
                        "If the exact entry is already present, check the effective sshd configuration:\n\n• PubkeyAuthentication yes\n• AuthorizedKeysFile includes the account’s authorized_keys file\n• The file and ~/.ssh have ownership and permissions accepted by sshd\n• No Match block overrides these settings\n• PubkeyAcceptedAlgorithms allows:\n  {}\nUse a leading + to add algorithms without replacing the defaults, for example:\n{}\n\nThe server’s SSH authentication log may identify the exact cause.",
                        report.public_key_algorithm,
                        report.openssh_algorithm_directives.join("\n"),
                    ),
                )),
            ),
            SshConnectionTestResult::ConnectionFailed(problem) => {
                ("SSH Connection Failed".to_owned(), problem.clone(), None)
            }
        };
        Self {
            heading,
            body,
            troubleshooting,
        }
    }
}
