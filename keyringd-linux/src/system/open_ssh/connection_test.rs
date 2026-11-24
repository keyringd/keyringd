use keyringd_core::service::{SshConnectionTest, SshConnectionTestResult};
const AGENT_SOCKET: &str = "/run/user/%i/keyringd-agent.sock";

pub(crate) async fn test_ssh_connection(
    host: &str,
    public_identity: &str,
) -> SshConnectionTestResult {
    let configuration = match run_ssh(&["-G", "--", host]).await {
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
    let environment = std::env::var("SSH_AUTH_SOCK").ok();
    let agent = match identity_agent {
        None | Some("SSH_AUTH_SOCK" | "$SSH_AUTH_SOCK" | "${SSH_AUTH_SOCK}") => {
            environment.as_deref()
        }
        value => value,
    };
    if !agent.is_some_and(is_keyringd_agent) {
        return SshConnectionTestResult::ClientConfigurationProblem(
            "SSH is not using Keyringd. Configure SSH_AUTH_SOCK and sign out and back in. Check ~/.ssh/config for an IdentityAgent override.".into(),
        );
    }

    let test = match SshConnectionTest::new(public_identity) {
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
    test.execute(tokio::process::Command::new("ssh"), host)
        .await
}

async fn run_ssh(arguments: &[&str]) -> std::io::Result<std::process::Output> {
    let mut command = tokio::process::Command::new("ssh");
    command.args(arguments);
    SshConnectionTest::output(command, std::time::Duration::from_secs(15)).await
}

fn is_keyringd_agent(value: &str) -> bool {
    value == AGENT_SOCKET || value.ends_with("/keyringd-agent.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "is_keyringd_agent" {
            case "Recognize a configured SSH Agent" {
                verify "Confirm that you know the configured SSH Agent" {
                    assert!(is_keyringd_agent(AGENT_SOCKET));
                    assert!(is_keyringd_agent(
                        "/run/user/1000/keyringd-agent.sock"
                    ));
                    assert!(!is_keyringd_agent("/run/user/1000/other-agent.sock"));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshConnectionTest::execute" {
            case "Real OpenSSH validates the selected identity even when another configured key can log in" {
                async: tokio;
                timeout: std::time::Duration::from_secs(30);
                verify "the selected key succeeds and a different successful key cannot pass the test" {
                    let directory = tempfile::tempdir().unwrap();
                    for name in ["selected", "other"] {
                        let mut command = tokio::process::Command::new("ssh-keygen");
                        command.args(["-q", "-t", "ed25519", "-N", "", "-f"]).arg(directory.path().join(name));
                        assert!(SshConnectionTest::output(command, std::time::Duration::from_secs(5)).await.unwrap().status.success());
                    }
                    let socket = directory.path().join("agent.sock");
                    let mut agent = tokio::process::Command::new("ssh-agent")
                        .args(["-D", "-a"]).arg(&socket)
                        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
                        .kill_on_drop(true).spawn().unwrap();
                    tokio::time::timeout(std::time::Duration::from_secs(5), async {
                        while !socket.exists() { tokio::time::sleep(std::time::Duration::from_millis(10)).await; }
                    }).await.unwrap();
                    let mut add = tokio::process::Command::new("ssh-add");
                    add.env("SSH_AUTH_SOCK", &socket).arg(directory.path().join("selected"));
                    assert!(SshConnectionTest::output(add, std::time::Duration::from_secs(5)).await.unwrap().status.success());
                    let selected = std::fs::read_to_string(directory.path().join("selected.pub")).unwrap();
                    let other = std::fs::read_to_string(directory.path().join("other.pub")).unwrap();
                    // sshd checks every parent directory of authorized_keys. Keep
                    // the server fixture under the user's home, not a shared /tmp.
                    let server_directory = tempfile::tempdir_in(std::env::var_os("HOME").unwrap()).unwrap();
                    for (authorized, should_pass) in [(&selected, true), (&other, false)] {
                        let mut server = keyringd_testkit::ssh::TestSshd::start_in(Some(server_directory.path()), authorized).await.unwrap();
                        let config = directory.path().join("config");
                        std::fs::write(&config, format!(
                            "Host target\nHostName 127.0.0.1\nPort {}\nIdentityAgent {}\nIdentityFile {}\nStrictHostKeyChecking no\nUserKnownHostsFile /dev/null\nGlobalKnownHostsFile /dev/null\n",
                            server.port(), socket.display(), directory.path().join("other").display())).unwrap();
                        let mut command = tokio::process::Command::new("ssh");
                        command.arg("-F").arg(&config);
                        let test = SshConnectionTest::new(&selected).unwrap();
                        let result = test.execute(command, "target").await;
                        if should_pass {
                            assert_eq!(result, SshConnectionTestResult::Connected, "{}", server.log());
                        } else {
                            assert_eq!(result, SshConnectionTestResult::SelectedIdentityUnconfirmed, "{}", server.log());
                            assert!(server.log().contains("Accepted publickey"), "another configured key must actually succeed: {}", server.log());
                        }
                        server.stop().await.unwrap();
                    }
                    agent.kill().await.unwrap();
                }
            }
        }
    }
}
