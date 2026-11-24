use super::{StartupMode, StartupModeError};
use clap::Parser;
use std::path::PathBuf;

fn default_data_dir() -> anyhow::Result<PathBuf> {
    if let Some(path) = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path).join("keyringd"));
    }
    let home = std::env::var_os("HOME")
        .ok_or_else(|| anyhow::anyhow!("HOME is not set. Specify --data-dir explicitly."))?;
    Ok(PathBuf::from(home).join(".local/share/keyringd"))
}

fn default_socket_path() -> Option<PathBuf> {
    std::env::var("XDG_RUNTIME_DIR")
        .ok()
        .map(|dir| PathBuf::from(dir).join("keyringd-agent.sock"))
}

#[derive(Parser)]
#[command(
    name = "keyringd",
    about = "SSH agent with hardware-protected device keys and passkey bootstrap"
)]
pub(crate) enum CommandLine {
    Agent {
        #[arg(long)]
        socket: Option<PathBuf>,
        #[arg(long)]
        data_dir: Option<PathBuf>,
        /// Accept nearby PXP BLE advertisements forwarded through the Agent relay extension.
        #[arg(long)]
        accept_remote_hybrid_advertisements: bool,
    },
    /// Ask the Agent to open the Profiles window.
    Settings,
    /// Forward nearby Hybrid BLE advertisements to an opted-in keyringd Agent.
    RelayHybridAdvertisements {
        /// Agent socket, including one forwarded from another machine.
        #[arg(long, env = "SSH_AUTH_SOCK")]
        agent_socket: PathBuf,
    },
}

impl CommandLine {
    pub(crate) fn resolve(self) -> anyhow::Result<StartupMode> {
        match self {
            Self::Agent {
                socket,
                data_dir,
                accept_remote_hybrid_advertisements,
            } => {
                let socket = socket
                    .or_else(default_socket_path)
                    .ok_or(StartupModeError)?;
                Ok(StartupMode::Agent {
                    socket,
                    data_dir: data_dir.map(Ok).unwrap_or_else(default_data_dir)?,
                    accept_remote_hybrid_advertisements,
                })
            }
            Self::Settings => Ok(StartupMode::Settings),
            Self::RelayHybridAdvertisements { agent_socket } => {
                Ok(StartupMode::RelayHybridAdvertisements { agent_socket })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "CommandLine::resolve" {
            case "Resolve the SSH Agent's PATH" {
                given "Prepare an Agent command line with Socket and Data Directory" {
                    let command = CommandLine::try_parse_from([
                        "keyringd",
                        "agent",
                        "--socket",
                        "/tmp/keyringd-test.sock",
                        "--data-dir",
                        "/tmp/keyringd-test-data",
                    ])
                    .unwrap();
                }
                when "Resolve command line to MODE" {
                    let mode = command.resolve().unwrap();
                }
                then "Become an agent mode with the specified PATH" {
                    assert!(matches!(
                        mode,
                        StartupMode::Agent {
                            socket,
                            data_dir,
                            accept_remote_hybrid_advertisements: false,
                        }
                            if socket == std::path::Path::new("/tmp/keyringd-test.sock")
                                && data_dir == std::path::Path::new("/tmp/keyringd-test-data")
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CommandLine::resolve" {
            case "Remote hybrid advertisements are opt in" {
                verify "leave the relay disabled unless the command line explicitly enables it" {
                    let command = CommandLine::try_parse_from([
                        "keyringd",
                        "agent",
                        "--socket",
                        "/tmp/keyringd-test.sock",
                        "--accept-remote-hybrid-advertisements",
                    ])
                    .unwrap();
                    assert!(matches!(
                        command.resolve().unwrap(),
                        StartupMode::Agent {
                            accept_remote_hybrid_advertisements: true,
                            ..
                        }
                    ));
                }
            }
        }
    }
}
