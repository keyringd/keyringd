use std::fs::File;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};

use super::ContainerCommand;
use super::TestEnvironment;
use crate::Distribution;
use crate::environment::testbed::TestbedExecutionRequest;
use crate::environment::testbed::virtual_bluetooth::VirtualBluetooth;

pub(crate) struct SystemContainer {
    name: String,
    pub(super) distribution: Distribution,
    pub(super) workspace: PathBuf,
    pub(super) package: Option<PathBuf>,
    pub(super) test_executable: PathBuf,
    pub(super) usb_control_socket: PathBuf,
    pub(super) report_directory: PathBuf,
    owner_uid: u32,
    owner_gid: u32,
    pub(super) authenticator_control_url: String,
    pub(super) filter: Option<String>,
    child: Option<Child>,
}

impl SystemContainer {
    pub(crate) fn boot(
        options: &TestbedExecutionRequest,
        usb_control_socket: &std::path::Path,
        bluetooth: Option<&VirtualBluetooth>,
        interrupted: &AtomicBool,
    ) -> Result<Self> {
        let runtime = options
            .workspace
            .join(&options.report_directory)
            .join("runtime");
        let log_path = runtime.join("nspawn.log");
        let stdout = File::create(&log_path)
            .with_context(|| format!("failed to create {}", log_path.display()))?;
        let stderr = stdout.try_clone()?;

        let mut command = Command::new("systemd-nspawn");
        command
            .arg("--quiet")
            .arg("--boot")
            .arg("--register=yes")
            .arg("--volatile=overlay")
            .arg("--resolv-conf=replace-host")
            .arg(format!("--machine={}", options.machine))
            .arg(format!("--image={}", options.image.display()))
            .arg(format!("--bind={}:/workspace", options.workspace.display()));
        command.arg(format!(
            "--bind={}:/run/keyringd-e2e-usb.sock",
            usb_control_socket.display()
        ));
        command.arg("--property=DeviceAllow=char-hidraw rw");
        if bluetooth.is_some() {
            command.arg("--capability=CAP_NET_ADMIN");
        }
        let child = command
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .context("failed to start the systemd-nspawn testbed")?;

        let mut container = Self {
            name: options.machine.clone(),
            distribution: options.distribution,
            workspace: options.workspace.clone(),
            package: options.package.as_ref().map(|path| container_path(path)),
            test_executable: container_path(&options.test_executable),
            usb_control_socket: PathBuf::from("/run/keyringd-e2e-usb.sock"),
            report_directory: container_path(&options.report_directory),
            owner_uid: options.owner_uid,
            owner_gid: options.owner_gid,
            authenticator_control_url: options.authenticator_control_url.clone(),
            filter: options.filter.clone(),
            child: Some(child),
        };
        let startup = (|| {
            container.wait_until_system_ready(interrupted)?;
            container.prepare_testbed()?;
            container.wait_until_user_ready(interrupted)
        })();
        if let Err(error) = startup {
            let cleanup = container.shutdown();
            crate::util::finish([
                ("system container startup", Err(error)),
                ("system container cleanup", cleanup),
            ])?;
        }
        Ok(container)
    }

    pub(crate) fn execute_tests(
        &mut self,
        interrupted: &AtomicBool,
        bluetooth: Option<&VirtualBluetooth>,
    ) -> Result<()> {
        TestEnvironment::new(self, bluetooth, interrupted).execute()
    }

    fn prepare_testbed(&self) -> Result<()> {
        let command = self.command();
        command.run_as_root(
            "container hostname resolution",
            [
                "sh",
                "-c",
                "printf '127.0.1.1 %s\\n' \"$(hostname)\" >> /etc/hosts",
            ],
        )?;
        let uid = self.owner_uid.to_string();
        let gid = self.owner_gid.to_string();
        command.run_as_root(
            "E2E user preparation",
            [
                "sh",
                "-eu",
                "-c",
                r#"
                    uid=$1
                    gid=$2

                    if ! getent group e2e >/dev/null; then
                        existing_group=$(getent group "$gid" | cut -d: -f1 || true)
                        if [ -n "$existing_group" ]; then
                            groupmod --new-name e2e "$existing_group"
                        else
                            groupadd --gid "$gid" e2e
                        fi
                    fi

                    if ! getent passwd e2e >/dev/null; then
                        existing_user=$(getent passwd "$uid" | cut -d: -f1 || true)
                        if [ -n "$existing_user" ]; then
                            usermod --login e2e --home /home/e2e --move-home "$existing_user"
                        else
                            useradd --create-home --uid "$uid" --gid e2e --shell /bin/bash e2e
                        fi
                    fi
                "#,
                "keyringd-e2e-user",
                &uid,
                &gid,
            ],
        )?;

        match self.distribution {
            Distribution::Ubuntu => command.run_as_root(
                "distribution SSH service masking",
                ["systemctl", "mask", "--now", "ssh.service", "ssh.socket"],
            )?,
            Distribution::Fedora => command.run_as_root(
                "distribution SSH service masking",
                ["systemctl", "mask", "--now", "sshd.service"],
            )?,
        }
        command.run_as_root(
            "E2E user manager enablement",
            ["loginctl", "enable-linger", "e2e"],
        )
    }

    fn wait_until_system_ready(&mut self, interrupted: &AtomicBool) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            self.ensure_running()?;
            if interrupted.load(Ordering::SeqCst) {
                bail!("interrupted while booting the system container");
            }
            let output = Command::new("systemctl")
                .arg(format!("--machine={}", self.name))
                .arg("is-system-running")
                .output();
            if output.is_ok_and(|output| {
                matches!(
                    String::from_utf8_lossy(&output.stdout).trim(),
                    "running" | "degraded"
                )
            }) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                self.print_diagnostics();
                bail!("container systemd did not become ready");
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn wait_until_user_ready(&mut self, interrupted: &AtomicBool) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            self.ensure_running()?;
            ensure!(!interrupted.load(Ordering::SeqCst), "interrupted");
            if self.command().root_command_succeeds([
                "systemctl",
                "is-active",
                "--quiet",
                &format!("user@{}.service", self.owner_uid),
            ])? {
                return Ok(());
            }
            if Instant::now() >= deadline {
                self.print_diagnostics();
                bail!("container user manager did not become ready");
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub(crate) fn print_diagnostics(&self) {
        let mut failed_units = Command::new("systemctl");
        failed_units
            .arg(format!("--machine={}", self.name))
            .args(["--failed", "--no-pager"]);
        print_diagnostic("failed units", &mut failed_units);

        let mut journal = Command::new("journalctl");
        journal.arg(format!("--machine={}", self.name)).args([
            "--boot",
            "--lines=100",
            "--no-pager",
        ]);
        print_diagnostic("recent journal", &mut journal);
    }

    pub(crate) fn shutdown(&mut self) -> Result<()> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        let mut failures = Vec::new();
        match Command::new("machinectl")
            .args(["poweroff", &self.name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) if status.success() => {}
            Ok(status) => failures.push((
                "machinectl poweroff",
                Err(anyhow::anyhow!("process exited with {status}")),
            )),
            Err(error) => failures.push(("execute machinectl poweroff", Err(error.into()))),
        }

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut exited = false;
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => {
                    exited = true;
                    break;
                }
                Ok(None) => thread::sleep(Duration::from_millis(100)),
                Err(error) => {
                    failures.push(("query systemd-nspawn status", Err(error.into())));
                    break;
                }
            }
        }
        if !exited {
            match Command::new("machinectl")
                .args(["terminate", &self.name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
                Ok(status) if status.success() => {}
                Ok(status) => failures.push((
                    "machinectl terminate",
                    Err(anyhow::anyhow!("process exited with {status}")),
                )),
                Err(error) => failures.push(("execute machinectl terminate", Err(error.into()))),
            }
            match child.try_wait() {
                Ok(Some(_)) => {}
                Ok(None) => {
                    if let Err(error) = child.kill() {
                        failures.push(("kill systemd-nspawn", Err(error.into())));
                    }
                }
                Err(error) => failures.push(("query systemd-nspawn status", Err(error.into()))),
            }
            if let Err(error) = child.wait() {
                failures.push(("reap systemd-nspawn", Err(error.into())));
            }
        }

        keyringd_testkit::runner::StageFailures::finish(failures)
    }

    fn ensure_running(&mut self) -> Result<()> {
        if let Some(status) = self
            .child
            .as_mut()
            .expect("the nspawn child should remain owned by the container while it is running")
            .try_wait()?
        {
            bail!("systemd-nspawn exited before the testbed was ready: {status}");
        }
        Ok(())
    }

    pub(super) fn command(&self) -> ContainerCommand<'_> {
        ContainerCommand::new(&self.name, self.owner_uid, self.owner_gid)
    }
}

fn container_path(relative: &std::path::Path) -> PathBuf {
    std::path::Path::new("/workspace").join(relative)
}

fn print_diagnostic(description: &str, command: &mut Command) {
    match command.output() {
        Ok(output) => {
            eprintln!("--- system container {description} ---");
            eprint!("{}", String::from_utf8_lossy(&output.stdout));
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
        Err(error) => eprintln!("failed to collect system container {description}: {error}"),
    }
}

impl Drop for SystemContainer {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
