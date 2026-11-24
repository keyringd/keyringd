use super::{
    ADAPTER_VISIBILITY_HELPER, HiddenBluetoothAdapters, PoweredOffVirtualBluetooth,
    RemoteHybridAdvertisements, reset_software_tpm,
};

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use anyhow::{Context, Result, anyhow, ensure};
use tempfile::TempDir;
use tokio::process::Command;
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

use crate::environment::linux::CommandOutput;

/// Filesystem and process environment owned by one Linux system test scenario.
///
/// `HOME` deliberately remains unchanged so the existing systemd user manager
/// can be exercised. Every application data path and every OpenSSH path used by
/// the scenario is rooted below `root` instead.
#[derive(Clone)]
pub struct Environment {
    root: PathBuf,
    _temporary_root: Option<Arc<TempDir>>,
    config_home: PathBuf,
    data_home: PathBuf,
    cache_home: PathBuf,
    artifacts: PathBuf,
    agent_socket: PathBuf,
    display: String,
    tpm_tcti: String,
}

impl Environment {
    /// Read only the case-owned product store, including the successful empty state.
    pub fn stored_profiles(&self) -> Result<Vec<serde_json::Value>> {
        let path = self.root.join("data/keyringd/profiles.json");
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let document: serde_json::Value = serde_json::from_slice(&bytes)?;
        document["profiles"]
            .as_array()
            .cloned()
            .context("profile store has no profiles array")
    }

    pub fn new() -> Result<Self> {
        let temporary_root = tempfile::Builder::new()
            .prefix("keyringd-linux-e2e-")
            .tempdir()
            .context("failed to create the E2E directory")?;
        let root = temporary_root.path().to_path_buf();
        Self::at(
            root,
            Some(Arc::new(temporary_root)),
            None,
            runtime_directory()?,
            std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_owned()),
            std::env::var("TPM2TOOLS_TCTI").unwrap_or_else(|_| "device".to_owned()),
        )
    }

    pub fn from_environment() -> Result<Self> {
        let root = std::env::var_os("KEYRINGD_E2E_ROOT")
            .context("KEYRINGD_E2E_ROOT is not set by the Linux system test launcher")?;
        let artifacts = std::env::var_os("KEYRINGD_E2E_ARTIFACTS").map(PathBuf::from);
        let display =
            std::env::var("DISPLAY").context("DISPLAY is required by the Linux system tests")?;
        let tpm_tcti = std::env::var("TPM2TOOLS_TCTI")
            .context("TPM2TOOLS_TCTI is required by the Linux system tests")?;
        let environment = Self::at(
            PathBuf::from(root),
            None,
            artifacts,
            runtime_directory()?,
            display,
            tpm_tcti,
        )?;
        initialize_trace(&environment.artifacts)?;
        Ok(environment)
    }

    fn at(
        root: PathBuf,
        temporary_root: Option<Arc<TempDir>>,
        artifacts: Option<PathBuf>,
        runtime_dir: PathBuf,
        display: String,
        tpm_tcti: String,
    ) -> Result<Self> {
        let config_home = root.join("config");
        let data_home = root.join("data");
        let cache_home = root.join("cache");
        let artifacts = artifacts.unwrap_or_else(|| root.join("artifacts"));
        let agent_socket = runtime_dir.join("keyringd-agent.sock");

        for directory in [&config_home, &data_home, &cache_home, &artifacts] {
            std::fs::create_dir_all(directory).with_context(|| {
                format!("failed to create E2E directory {}", directory.display())
            })?;
        }

        let environment = Self {
            root,
            _temporary_root: temporary_root,
            config_home,
            data_home,
            cache_home,
            artifacts,
            agent_socket,
            display,
            tpm_tcti,
        };
        environment.assert_isolated()?;
        Ok(environment)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn home(&self) -> Result<PathBuf> {
        std::env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .context("HOME is required by the installed application")
    }

    pub fn artifacts(&self) -> &Path {
        &self.artifacts
    }

    pub fn artifact(&self, name: impl AsRef<Path>) -> PathBuf {
        self.artifacts.join(name)
    }

    pub fn agent_socket(&self) -> &Path {
        &self.agent_socket
    }

    pub fn command(&self, program: impl AsRef<OsStr>) -> Command {
        let mut command = Command::new(program);
        command
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_CACHE_HOME", &self.cache_home)
            .env("DISPLAY", &self.display)
            .env("GDK_BACKEND", "x11")
            .env_remove("WAYLAND_DISPLAY")
            .env("SSH_AUTH_SOCK", &self.agent_socket)
            .env("TPM2TOOLS_TCTI", &self.tpm_tcti)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    pub async fn run(
        &self,
        program: impl AsRef<OsStr>,
        arguments: &[&str],
    ) -> Result<CommandOutput> {
        let program = program.as_ref();
        let start = Instant::now();
        let output = self
            .command(program)
            .args(arguments)
            .output()
            .await
            .with_context(|| format!("failed to execute {}", program.to_string_lossy()))?;
        tracing::trace!(
            program = %program.to_string_lossy(),
            elapsed_ms = start.elapsed().as_millis(),
            success = output.status.success(),
            "E2E command completed"
        );
        Ok(CommandOutput::from(output))
    }

    pub fn assert_isolated(&self) -> Result<()> {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        for path in [
            &self.config_home,
            &self.data_home,
            &self.cache_home,
            &self.artifacts,
        ] {
            if path != &self.artifacts {
                ensure!(
                    path.starts_with(&self.root),
                    "E2E path escaped its temporary root: {}",
                    path.display()
                );
            }
            if let Some(home) = &home {
                ensure!(
                    !path.starts_with(home),
                    "E2E path overlaps the user's HOME: {}",
                    path.display()
                );
            }
        }
        ensure!(
            self.agent_socket.file_name() == Some(OsStr::new("keyringd-agent.sock")),
            "unexpected agent socket path: {}",
            self.agent_socket.display()
        );
        Ok(())
    }
}

fn initialize_trace(artifacts: &Path) -> Result<()> {
    let Some(filter) = std::env::var_os("KEYRINGD_E2E_TRACE") else {
        return Ok(());
    };
    static TRACE_INIT: OnceLock<std::result::Result<(), String>> = OnceLock::new();
    TRACE_INIT
        .get_or_init(|| {
            let filter = filter.to_str().ok_or("non-UTF-8 trace filter")?;
            let filter = EnvFilter::try_new(filter).map_err(|error| error.to_string())?;
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(artifacts.join("e2e-trace.log"))
                .map_err(|error| error.to_string())?;
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_span_events(FmtSpan::CLOSE)
                .with_writer(move || file.try_clone().expect("the open E2E trace log should remain clonable while the tracing subscriber uses it"))
                .with_ansi(false)
                .try_init()
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|message| anyhow!("failed to initialize E2E trace logging: {message}"))
        .map(|_| ())
}

fn runtime_directory() -> Result<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .context("XDG_RUNTIME_DIR is required for the systemd user socket")
}

use keyringd_testkit::agent::AgentIdentity;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

impl Environment {
    /// Reads the actual identities response, distinguishing an empty list from
    /// connection failure or an Agent failure response.
    pub async fn list_agent_identities(&self) -> Result<Vec<AgentIdentity>> {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            let mut stream = UnixStream::connect(self.agent_socket())
                .await
                .context("connect to SSH Agent")?;
            stream.write_all(&[0, 0, 0, 1, 11]).await?;
            let length = stream.read_u32().await?;
            ensure!(
                (5..=1024 * 1024).contains(&length),
                "invalid identities response frame size: {length}"
            );
            let mut response = vec![0; length as usize];
            stream.read_exact(&mut response).await?;
            ensure!(
                response[0] == 12,
                "Agent did not return an identities answer: {}",
                response[0]
            );
            let mut remaining = &response[1..];
            let count = read_u32(&mut remaining)?;
            ensure!(
                count <= remaining.len() as u32 / 8,
                "invalid identity count: {count}"
            );
            let mut identities = Vec::new();
            for _ in 0..count {
                let public_key = read_string(&mut remaining)?.to_vec();
                let mut key = public_key.as_slice();
                let algorithm = std::str::from_utf8(read_string(&mut key)?)?.to_owned();
                let comment = std::str::from_utf8(read_string(&mut remaining)?)?.to_owned();
                identities.push(AgentIdentity {
                    algorithm,
                    public_key,
                    comment,
                });
            }
            ensure!(
                remaining.is_empty(),
                "unexpected trailing identities response data"
            );
            Ok(identities)
        })
        .await
        .context("timed out listing SSH Agent identities")?
    }
}

fn read_u32(bytes: &mut &[u8]) -> Result<u32> {
    ensure!(bytes.len() >= 4, "truncated SSH Agent uint32");
    let (value, rest) = bytes.split_at(4);
    *bytes = rest;
    Ok(u32::from_be_bytes(value.try_into()?))
}

fn read_string<'a>(bytes: &mut &'a [u8]) -> Result<&'a [u8]> {
    let length = read_u32(bytes)? as usize;
    ensure!(length <= bytes.len(), "truncated SSH Agent string");
    let (value, rest) = bytes.split_at(length);
    *bytes = rest;
    Ok(value)
}

impl Environment {
    /// Exercise Linux's HCI detection with an empty directory in the container.
    /// Host controllers and BlueZ's D-Bus objects are not removed.
    pub async fn without_bluetooth_adapters(&self) -> Result<HiddenBluetoothAdapters> {
        let output = self
            .run("sudo", &["-n", ADAPTER_VISIBILITY_HELPER, "hide"])
            .await?;
        ensure!(
            output.status.success(),
            "hiding adapters failed: {}",
            output.stderr
        );
        Ok(HiddenBluetoothAdapters { restored: false })
    }

    /// Power off the PC adapter so the application sees BluetoothDisabled.
    pub async fn power_off_virtual_bluetooth(&self) -> Result<PoweredOffVirtualBluetooth> {
        let scanner = virtual_bluetooth_controller("KEYRINGD_E2E_BLE_SCANNER")?;
        let guard = PoweredOffVirtualBluetooth {
            controller: scanner,
            restored: false,
        };
        let controller = &guard.controller;
        let output = self.run_virtual_btmgmt(controller, "power off").await?;
        ensure!(
            output.status.success(),
            "powering off {controller} failed: {}",
            output.stderr
        );
        Ok(guard)
    }

    pub(crate) async fn run_virtual_btmgmt(
        &self,
        controller: &str,
        command: &str,
    ) -> Result<CommandOutput> {
        ensure!(
            controller.starts_with("hci")
                && controller.len() > 3
                && controller[3..].bytes().all(|byte| byte.is_ascii_digit()),
            "invalid virtual Bluetooth controller: {controller}"
        );
        ensure!(
            command
                .bytes()
                .all(|byte| { byte.is_ascii_alphanumeric() || matches!(byte, b' ' | b'-') }),
            "invalid virtual Bluetooth command"
        );
        self.run(
            "timeout",
            &[
                "-k",
                "2s",
                "5s",
                "script",
                "-qefc",
                &format!("sudo -n btmgmt -i {controller} {command}"),
                "/dev/null",
            ],
        )
        .await
    }
}

fn virtual_bluetooth_controller(variable: &str) -> Result<String> {
    let controller = std::env::var(variable).with_context(|| format!("{variable} is not set"))?;
    ensure!(
        controller.starts_with("hci")
            && controller.len() > 3
            && controller[3..].bytes().all(|byte| byte.is_ascii_digit()),
        "invalid btvirt controller in {variable}: {controller}"
    );
    Ok(controller)
}

use std::time::Duration;

impl Environment {
    /// Stop the activation source in the same transaction as the application.
    /// Otherwise a queued agent connection can replace the stop job with a start.
    pub(crate) async fn stop_installed_application(&self) -> Result<()> {
        let output = self
            .systemctl_user(&["stop", "keyringd-agent.socket", "keyringd-desktop.service"])
            .await?;
        ensure!(
            output.status.success(),
            "stopping the installed application failed: {}",
            output.stderr
        );
        Ok(())
    }

    pub async fn launch_installed_application(&self) -> Result<()> {
        let output = self
            .run("gapplication", &["launch", "com.keyringd.Keyringd"])
            .await?;
        if !output.status.success() {
            let journal = match self
                .run(
                    "journalctl",
                    &[
                        "--user",
                        "--unit=keyringd-desktop.service",
                        "--lines=30",
                        "--no-pager",
                        "--output=short-precise",
                    ],
                )
                .await
            {
                Ok(journal) => format!("{}{}", journal.stdout, journal.stderr),
                Err(error) => format!("{error:#}"),
            };
            anyhow::bail!(
                "gapplication launch failed: {}\nKeyringd service journal before cleanup:\n{journal}",
                output.stderr.trim_end()
            );
        }
        Ok(())
    }

    pub async fn reset_installed_application(&self) -> Result<()> {
        self.stop_installed_application().await?;
        reset_software_tpm(self, &self.tpm_tcti, &self.root).await?;
        for directory in [&self.config_home, &self.data_home, &self.cache_home] {
            if directory.exists() {
                std::fs::remove_dir_all(directory).with_context(|| {
                    format!("failed to reset E2E directory {}", directory.display())
                })?;
            }
            std::fs::create_dir_all(directory).with_context(|| {
                format!("failed to recreate E2E directory {}", directory.display())
            })?;
        }
        Ok(())
    }

    pub async fn use_remote_hybrid_advertisements(&self) -> Result<RemoteHybridAdvertisements> {
        let drop_in = self
            .home()?
            .join(".config/systemd/user/keyringd-desktop.service.d/remote-advertisements.conf");
        ensure!(
            !drop_in.exists(),
            "remote Hybrid advertisement option leaked from another case"
        );
        self.stop_installed_application().await?;
        std::fs::create_dir_all(drop_in.parent().context("Agent drop-in has no parent")?)?;
        std::fs::copy(
            "/workspace/tests/keyringd-system-tests-linux-runner/testbed/keyringd-remote-advertisements.conf",
            &drop_in,
        )?;
        let guard = RemoteHybridAdvertisements {
            drop_in,
            restored: false,
        };
        let output = self.systemctl_user(&["daemon-reload"]).await?;
        ensure!(
            output.status.success(),
            "systemd user reload failed: {}",
            output.stderr
        );
        Ok(guard)
    }

    pub async fn assert_installed_application_is_isolated(&self) -> Result<()> {
        let mut main_pid = None;
        for _ in 0..100 {
            let output = self
                .systemctl_user(&[
                    "show",
                    "--property=MainPID",
                    "--value",
                    "keyringd-desktop.service",
                ])
                .await?;
            ensure!(
                output.status.success(),
                "systemctl show failed: {}",
                output.stderr
            );
            if let Ok(pid) = output.stdout.trim().parse::<u32>()
                && pid != 0
            {
                main_pid = Some(pid);
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let main_pid = main_pid.context("installed application did not acquire a MainPID")?;
        let environment =
            std::fs::read(format!("/proc/{main_pid}/environ")).with_context(|| {
                format!("failed to read environment for application PID {main_pid}")
            })?;
        let variables = environment
            .split(|byte| *byte == 0)
            .filter_map(|entry| std::str::from_utf8(entry).ok())
            .collect::<Vec<_>>();

        for (name, expected) in [
            ("DISPLAY", self.display.as_str()),
            ("GDK_BACKEND", "x11"),
            (
                "XDG_CONFIG_HOME",
                self.config_home.to_str().context("non-UTF-8 config path")?,
            ),
            (
                "XDG_DATA_HOME",
                self.data_home.to_str().context("non-UTF-8 data path")?,
            ),
            (
                "XDG_CACHE_HOME",
                self.cache_home.to_str().context("non-UTF-8 cache path")?,
            ),
            ("TPM2TOOLS_TCTI", self.tpm_tcti.as_str()),
        ] {
            let assignment = format!("{name}={expected}");
            ensure!(
                variables.iter().any(|variable| *variable == assignment),
                "installed application PID {main_pid} does not have {assignment}"
            );
        }
        ensure!(
            !variables
                .iter()
                .any(|variable| variable.starts_with("WAYLAND_DISPLAY=")),
            "installed application PID {main_pid} can escape through WAYLAND_DISPLAY"
        );
        Ok(())
    }

    pub async fn systemctl_user(&self, arguments: &[&str]) -> Result<CommandOutput> {
        let mut command_arguments = Vec::with_capacity(arguments.len() + 1);
        command_arguments.push("--user");
        command_arguments.extend_from_slice(arguments);
        self.run("systemctl", &command_arguments).await
    }

    pub async fn agent_service_active_state(&self) -> Result<String> {
        let output = self
            .run(
                "systemctl",
                &[
                    "--user",
                    "show",
                    "--property=ActiveState",
                    "--value",
                    "keyringd-desktop.service",
                ],
            )
            .await?;
        ensure!(
            output.status.success(),
            "systemctl show failed: {}",
            output.stderr
        );
        Ok(output.stdout.trim().to_owned())
    }

    pub async fn agent_journal(&self) -> Result<String> {
        let output = self
            .run(
                "journalctl",
                &[
                    "--user",
                    "--unit=keyringd-desktop.service",
                    "--since=-10 minutes",
                    "--no-pager",
                    "--output=short-precise",
                ],
            )
            .await?;
        Ok(format!("{}{}", output.stdout, output.stderr))
    }
}

#[cfg(test)]
mod tests {
    use super::Environment;

    keyringd_testkit::scenario! {
        group "Environment::at" {
            case "Scenario paths are isolated from the user home" {
                verify "keep scenario state beneath the isolated test root" {
                    let root = tempfile::tempdir().unwrap();
                    let runtime = tempfile::tempdir().unwrap();
                    let environment = Environment::at(
                        root.path().to_path_buf(),
                        None,
                        None,
                        runtime.path().to_path_buf(),
                        ":0".to_owned(),
                        "device".to_owned(),
                    )
                    .unwrap();

                    environment.assert_isolated().unwrap();
                    assert!(environment.artifacts().starts_with(environment.root()));
                    assert_eq!(
                        environment
                            .agent_socket()
                            .file_name()
                            .and_then(|name| name.to_str()),
                        Some("keyringd-agent.sock")
                    );
                }
            }
        }
    }
}
