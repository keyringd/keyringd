use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, ensure};

#[derive(Clone, Copy)]
pub(super) struct ContainerCommand<'a> {
    machine: &'a str,
    uid: u32,
    gid: u32,
}

impl<'a> ContainerCommand<'a> {
    pub(super) fn new(machine: &'a str, uid: u32, gid: u32) -> Self {
        Self { machine, uid, gid }
    }

    pub(super) fn run_as_root(
        self,
        description: &str,
        arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<()> {
        self.run_as_root_with_options(description, std::iter::empty::<&str>(), arguments)
    }

    pub(super) fn run_as_root_with_options(
        self,
        description: &str,
        options: impl IntoIterator<Item = impl AsRef<OsStr>>,
        arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<()> {
        let status = self
            .systemd_run()
            .args(options)
            .arg("--")
            .args(arguments)
            .status()
            .with_context(|| format!("failed to start {description} in the system container"))?;
        ensure!(status.success(), "{description} failed with {status}");
        Ok(())
    }

    pub(super) fn run_as_root_with_timeout(
        self,
        description: &str,
        timeout: Duration,
        arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<()> {
        let service_timeout = format!("{}s", timeout.as_secs());
        let client_timeout = format!("{}s", (timeout + Duration::from_secs(10)).as_secs());
        let status = Command::new("timeout")
            .args(["--kill-after=5s", &client_timeout, "systemd-run"])
            .arg(format!("--machine={}", self.machine))
            .args(["--wait", "--pipe", "--collect", "--service-type=exec"])
            .arg(format!("--property=RuntimeMaxSec={service_timeout}"))
            .arg("--")
            .args(arguments)
            .status()
            .with_context(|| format!("failed to start {description} in the system container"))?;
        ensure!(
            status.success(),
            "{description} failed with {status} (service deadline {service_timeout}, client deadline {client_timeout})"
        );
        Ok(())
    }

    pub(super) fn root_command_succeeds(
        self,
        arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<bool> {
        Ok(self
            .systemd_run()
            .arg("--quiet")
            .arg("--")
            .args(arguments)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?
            .success())
    }

    pub(super) fn run_as_e2e(
        self,
        description: &str,
        arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<()> {
        self.run_as_e2e_with_options(description, std::iter::empty::<&str>(), arguments)
    }

    pub(super) fn run_as_e2e_with_options(
        self,
        description: &str,
        options: impl IntoIterator<Item = impl AsRef<OsStr>>,
        arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<()> {
        let status = self
            .e2e_command()
            .args(options)
            .arg("--")
            .args(arguments)
            .status()
            .with_context(|| format!("failed to start {description} in the test environment"))?;
        ensure!(status.success(), "{description} failed with {status}");
        Ok(())
    }

    pub(super) fn output_as_e2e_with_options(
        self,
        description: &str,
        options: impl IntoIterator<Item = impl AsRef<OsStr>>,
        arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<std::process::Output> {
        let output = self
            .e2e_command()
            .arg("--quiet")
            .args(options)
            .arg("--")
            .args(arguments)
            .output()
            .with_context(|| format!("failed to start {description} in the test environment"))?;
        ensure!(
            output.status.success(),
            "{description} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(output)
    }

    pub(super) fn start_e2e_service(
        self,
        description: &str,
        unit: &str,
        log: &Path,
        arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<()> {
        let status = Command::new("systemd-run")
            .arg(format!("--machine={}", self.machine))
            .arg(format!("--unit={unit}"))
            .arg(format!("--uid={}", self.uid))
            .arg(format!("--gid={}", self.gid))
            .args(["--service-type=exec", "--collect"])
            .arg(format!(
                "--property=StandardOutput=append:{}",
                log.display()
            ))
            .arg(format!("--property=StandardError=append:{}", log.display()))
            .arg("--")
            .args(arguments)
            .status()
            .with_context(|| format!("failed to start {description} in the test environment"))?;
        ensure!(status.success(), "{description} failed with {status}");
        Ok(())
    }

    pub(super) fn e2e_command_succeeds(
        self,
        arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<bool> {
        Ok(self
            .e2e_command()
            .arg("--quiet")
            .arg("--")
            .args(arguments)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?
            .success())
    }

    fn e2e_command(self) -> Command {
        let mut command = self.systemd_run();
        command
            .arg(format!("--uid={}", self.uid))
            .arg(format!("--gid={}", self.gid))
            .arg("--working-directory=/workspace")
            .arg("--setenv=HOME=/home/e2e")
            .arg(format!("--setenv=XDG_RUNTIME_DIR=/run/user/{}", self.uid))
            .arg(format!(
                "--setenv=DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/{}/bus",
                self.uid
            ));
        command
    }

    fn systemd_run(self) -> Command {
        let mut command = Command::new("systemd-run");
        command.arg(format!("--machine={}", self.machine)).args([
            "--wait",
            "--pipe",
            "--collect",
            "--service-type=exec",
        ]);
        command
    }
}
