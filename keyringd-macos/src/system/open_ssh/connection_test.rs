use keyringd_core::service::{SshConnectionTest, SshConnectionTestResult};

#[cfg(feature = "test-support")]
pub(crate) static FIXTURE_SSH_CONFIG: std::sync::OnceLock<std::path::PathBuf> =
    std::sync::OnceLock::new();

pub(crate) async fn test_open_ssh_connection(
    host: String,
    socket_path: String,
    public_identity: String,
) -> SshConnectionTestResult {
    let environment = match super::MacOsSshAgentConfiguration::selected_socket() {
        Ok(socket) => socket,
        Err(error) => {
            return SshConnectionTestResult::ClientConfigurationProblem(
                keyringd_core::util::report_user_error(
                    &error,
                    "Could not read the login session's SSH_AUTH_SOCK setting.",
                ),
            );
        }
    };
    let ssh = || {
        let mut command = tokio::process::Command::new("/usr/bin/ssh");
        #[cfg(feature = "test-support")]
        if let Some(config) = FIXTURE_SSH_CONFIG.get() {
            command.arg("-F").arg(if config.exists() {
                config.as_path()
            } else {
                std::path::Path::new("/dev/null")
            });
        }
        match &environment {
            Some(socket) => command.env("SSH_AUTH_SOCK", socket),
            None => command.env_remove("SSH_AUTH_SOCK"),
        };
        command
    };
    let configuration_output = {
        let mut command = ssh();
        command.args(["-G", "--", &host]);
        SshConnectionTest::output(command, std::time::Duration::from_secs(15)).await
    };
    let configuration = match configuration_output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).into_owned()
        }
        Ok(output) => {
            return SshConnectionTestResult::ClientConfigurationProblem(
                keyringd_core::util::report_user_error(
                    &String::from_utf8_lossy(&output.stderr),
                    "Could not read the SSH configuration. Check the Host and Include rules in your SSH configuration, then try again.",
                ),
            );
        }
        Err(error) => {
            return SshConnectionTestResult::ClientConfigurationProblem(
                keyringd_core::util::report_user_error(
                    &error,
                    "Could not start OpenSSH. Check that OpenSSH is installed and available, then try again.",
                ),
            );
        }
    };
    let identity_agent = configuration.lines().find_map(|line| {
        let (key, value) = line.split_once(' ')?;
        key.eq_ignore_ascii_case("identityagent")
            .then(|| value.trim())
    });
    let agent = match identity_agent {
        None | Some("SSH_AUTH_SOCK" | "$SSH_AUTH_SOCK" | "${SSH_AUTH_SOCK}") => {
            environment.as_deref()
        }
        value => value,
    };
    let uses_keyringd = match agent.filter(|value| !value.is_empty() && *value != "none") {
        Some(agent) => match super::agent_socket_matches::agent_socket_matches(
            std::path::Path::new(agent),
            std::path::Path::new(&socket_path),
        ) {
            Ok(matches) => matches,
            Err(error) => {
                return SshConnectionTestResult::ClientConfigurationProblem(format!(
                    "Could not compare the SSH agent socket paths: {error}\n\nSSH agent socket: {agent:?}\nkeyringd socket: {socket_path:?}",
                ));
            }
        },
        None => false,
    };
    if !uses_keyringd {
        return SshConnectionTestResult::ClientConfigurationProblem(format!(
            "SSH is not using Keyringd. Open Configure SSH Clients to select Keyringd. Check ~/.ssh/config for an IdentityAgent override.\n\nLogin session SSH_AUTH_SOCK: {}\nIdentityAgent: {}\nkeyringd socket: {}",
            environment.as_deref().unwrap_or("(not set)"),
            identity_agent.unwrap_or("(use SSH_AUTH_SOCK)"),
            socket_path,
        ));
    }

    let test = match SshConnectionTest::new(&public_identity) {
        Ok(test) => test,
        Err(error) => {
            return SshConnectionTestResult::ConnectionFailed(
                keyringd_core::util::report_user_error(
                    &error,
                    "Could not prepare the SSH connection test.",
                ),
            );
        }
    };
    test.execute(ssh(), &host).await
}
