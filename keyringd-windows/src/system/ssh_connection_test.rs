use keyringd_core::service::SshConnectionTest;
pub(crate) use keyringd_core::service::SshConnectionTestResult as ConnectionTestResult;
use std::time::Duration;
use tokio::process::Command;
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};
use windows::core::w;

use super::agent_pipe_name;
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub(crate) fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && !host.starts_with('-')
        && !host.chars().any(|c| c.is_whitespace() || c.is_control())
}

// Read the saved user setting as well, since this tray process may predate Configure Windows SSH.
fn agent_environment() -> Option<String> {
    let mut value = vec![0u16; 32768];
    let mut size = (value.len() * 2) as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Environment"),
            w!("SSH_AUTH_SOCK"),
            RRF_RT_REG_SZ,
            None,
            Some(value.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if status.is_ok() {
        let end = value.iter().position(|c| *c == 0).unwrap_or(value.len());
        Some(String::from_utf16_lossy(&value[..end]))
    } else {
        std::env::var("SSH_AUTH_SOCK").ok()
    }
}

fn command(agent: Option<&str>) -> Command {
    let mut command = Command::new("ssh.exe");
    command.creation_flags(CREATE_NO_WINDOW);
    if let Some(agent) = agent {
        command.env("SSH_AUTH_SOCK", agent);
    }
    command
}

fn configured_agent_at(configuration: &str, environment: Option<&str>, pipe: &str) -> bool {
    let configured = configuration.lines().find_map(|line| {
        let (key, value) = line.split_once(' ')?;
        key.eq_ignore_ascii_case("identityagent")
            .then_some(value.trim())
    });
    match configured {
        None | Some("SSH_AUTH_SOCK") | Some("$SSH_AUTH_SOCK") => environment == Some(pipe),
        Some(value) => value == pipe || value == pipe.replace('\\', "/"),
    }
}

pub(crate) async fn test(host: String, public_identity: String) -> ConnectionTestResult {
    let pipe = match agent_pipe_name() {
        Ok(pipe) => pipe,
        Err(error) => {
            return ConnectionTestResult::ClientConfigurationProblem(format!(
                "Could not determine the Windows agent pipe: {error}"
            ));
        }
    };
    test_with_configuration(host, public_identity, None, pipe, agent_environment()).await
}

#[cfg(feature = "test-support")]
pub(crate) async fn test_configured(
    host: String,
    public_identity: String,
    config: String,
    pipe: String,
) -> ConnectionTestResult {
    test_with_configuration(
        host,
        public_identity,
        Some(config),
        pipe.clone(),
        Some(pipe),
    )
    .await
}

async fn test_with_configuration(
    host: String,
    public_identity: String,
    config_file: Option<String>,
    pipe: String,
    agent: Option<String>,
) -> ConnectionTestResult {
    let make_command = || {
        let mut result = command(agent.as_deref());
        if let Some(config) = &config_file {
            result.args(["-F", config]);
        }
        result
    };
    use ConnectionTestResult::*;
    if !valid_host(&host) {
        return ClientConfigurationProblem(
            "Enter a hostname, user@host, or a Host alias from your SSH config, without command-line options.".into(),
        );
    }
    let mut lookup = make_command();
    lookup.args(["-G", "--", &host]);
    let config = match SshConnectionTest::output(lookup, Duration::from_secs(15)).await {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).into_owned()
        }
        Ok(output) => {
            return ClientConfigurationProblem(keyringd_core::util::report_user_error(
                &String::from_utf8_lossy(&output.stderr),
                "Could not read the SSH configuration. Check the Host and Include rules in your SSH configuration, then try again.",
            ));
        }
        Err(error) if error.kind() != std::io::ErrorKind::TimedOut => {
            return ClientConfigurationProblem(keyringd_core::util::report_user_error(
                &error,
                "Could not start OpenSSH. Check that OpenSSH is installed and available, then try again.",
            ));
        }
        Err(_) => {
            return ClientConfigurationProblem("OpenSSH configuration lookup timed out.".into());
        }
    };
    if !configured_agent_at(&config, agent.as_deref(), &pipe) {
        return ClientConfigurationProblem("OpenSSH is not configured to use keyringd. Use Configure Windows SSH, or check IdentityAgent in your SSH configuration.".into());
    }
    let test = match SshConnectionTest::new(&public_identity) {
        Ok(test) => test,
        Err(error) => {
            return ConnectionFailed(keyringd_core::util::report_user_error(
                &error,
                "Could not prepare the SSH connection test. Check that the temporary folder is writable and has available space, then try again.",
            ));
        }
    };
    test.execute(make_command(), &host).await
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "valid_host" {
            case "Accept the SSH destination and reject the command line OPTION" {
                given "Prepare a connection and a normal connection with empty characters, OPTION injections, and extra arguments" {
                    let invalid = ["", "-oProxyCommand=cmd", "user@host extra", "host\n"];
                    let valid = ["work", "user@host", "user@[::1]"];
                }
                when "Validate each string as an SSH destination" {
                    let invalid_results = invalid.map(valid_host);
                    let valid_results = valid.map(valid_host);
                }
                then "Accept only the destination and reject OPTION or additional arguments" {
                    assert!(invalid_results.into_iter().all(|result| !result));
                    assert!(valid_results.into_iter().all(|result| result));
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "configured_agent" {
            case "Prioritize the specified SSH Agent settings over preferences" {
                given "Prepare a different IdentityAgent specification for environment variables and ssh config" {
                    let pipe = agent_pipe_name().unwrap();
                    let explicit_agent = format!("identityagent {pipe}");
                }
                when "Determine if Keyringd Agent is configured in each setting" {
                    let environment_only = configured_agent_at("", Some(&pipe), &pipe);
                    let config_only = configured_agent_at(&explicit_agent, None, &pipe);
                    let disabled = configured_agent_at("identityagent none", Some(&pipe), &pipe);
                    let overridden = configured_agent_at("identityagent other", Some(&pipe), &pipe);
                }
                then "SSH Config explicit specification takes precedence over environment settings" {
                    assert!(environment_only);
                    assert!(config_only);
                    assert!(!disabled);
                    assert!(!overridden);
                }
            }
        }
    }
}
