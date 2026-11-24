use crate::WslClient;
use anyhow::{Context, Result, ensure};
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

pub struct WslFixture {
    distribution: String,
    directory: String,
    session: Option<Child>,
}

impl WslFixture {
    pub fn start(
        trust: &str,
        identity: &str,
        app: &mut keyringd_system_tests::environment::windows::TestApp,
    ) -> Result<Self> {
        let environment = app.stop_for_socket_activation()?;
        let distribution = std::env::var("KEYRINGD_TEST_WSL_DISTRO")?;
        let mut fixture = Self {
            distribution,
            directory: format!("/root/keyringd-e2e-{}", uuid::Uuid::new_v4()),
            session: None,
        };
        let script = include_str!("wsl_fixture/start.sh").replace("\r\n", "\n");
        let mut command = fixture.command();
        let mut forwarded = std::env::var("WSLENV").unwrap_or_default();
        for (name, value) in environment {
            if !forwarded.is_empty() {
                forwarded.push(':');
            }
            forwarded.push_str(&name);
            command.env(name, value);
        }
        command.env("WSLENV", forwarded);
        fixture.session = Some(
            command
                .args([
                    "bash",
                    "-c",
                    &script,
                    "wsl-fixture",
                    &fixture.directory,
                    trust,
                    identity,
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()?,
        );
        let stdout = fixture.session.as_mut().unwrap().stdout.take().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = (|| -> std::io::Result<()> {
                for line in BufReader::new(stdout).lines() {
                    let line = line?;
                    if line == "KEYRINGD_WSL_READY" {
                        return Ok(());
                    }
                    eprintln!("WSL setup: {line}");
                }
                Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "WSL setup exited before reporting readiness",
                ))
            })();
            let _ = sender.send(result);
        });
        // Include cold WSL startup in one timeout; observe the original session
        // rather than launching additional wsl.exe processes to poll a file.
        match receiver.recv_timeout(Duration::from_secs(60)) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => anyhow::bail!(
                "WSL fixture failed during setup: {error}; {}",
                fixture.logs().unwrap_or_default()
            ),
            Err(error) => anyhow::bail!(
                "WSL fixture did not become ready within 60 seconds: {error}; {}",
                fixture.logs().unwrap_or_default()
            ),
        }
        Ok(fixture)
    }

    fn command(&self) -> Command {
        let mut command = Command::new("wsl.exe");
        command.args([
            "--distribution",
            &self.distribution,
            "--user",
            "root",
            "--exec",
        ]);
        command
    }

    fn checked(&self, arguments: &[&str]) -> Result<Output> {
        let output = self.command().args(arguments).output()?;
        ensure!(
            output.status.success(),
            "WSL fixture failed: {}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
            self.logs().unwrap_or_default()
        );
        Ok(output)
    }

    pub fn identities(&self) -> Result<String> {
        let output = self.checked(&[
            "env",
            "SSH_AUTH_SOCK=/run/user/0/keyringd-agent.sock",
            "timeout",
            "15",
            "ssh-add",
            "-L",
        ])?;
        Ok(String::from_utf8(output.stdout)?)
    }

    pub fn start_client(&self) -> Result<WslClient> {
        let script = include_str!("wsl_fixture/client.sh").replace("\r\n", "\n");
        let child = self
            .command()
            .args(["bash", "-c", &script, "wsl-ssh-client", &self.directory])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("start WSL OpenSSH")?;
        Ok(WslClient { child: Some(child) })
    }

    pub fn assert_connected(&self, output: &Output) -> Result<()> {
        ensure!(
            output.status.success() && output.stdout == b"keyringd-wsl-ok",
            "WSL SSH authentication failed: {}\n{}",
            String::from_utf8_lossy(&output.stderr),
            self.logs().unwrap_or_default()
        );
        ensure!(
            self.logs()?.contains("Accepted publickey"),
            "sshd did not accept public-key authentication"
        );
        let output = self.checked(&[
            "env",
            "XDG_RUNTIME_DIR=/run/user/0",
            "systemctl",
            "--user",
            "show",
            "keyringd.service",
            "--property=ActiveState",
            "--value",
        ])?;
        ensure!(
            String::from_utf8(output.stdout)?.trim() == "active",
            "SSH did not activate the packaged proxy service"
        );
        Ok(())
    }

    pub fn logs(&self) -> Result<String> {
        let script = include_str!("wsl_fixture/logs.sh").replace("\r\n", "\n");
        let output = self
            .command()
            .args(["bash", "-c", &script, "logs", &self.directory])
            .output()?;
        ensure!(
            output.status.success(),
            "could not collect WSL fixture logs: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(String::from_utf8(output.stdout)?)
    }
}

impl Drop for WslFixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            match self.logs() {
                Ok(logs) => eprintln!("WSL fixture at failure:\n{logs}"),
                Err(error) => eprintln!("Could not collect failed WSL fixture logs: {error}"),
            }
        }
        let script = include_str!("wsl_fixture/stop.sh").replace("\r\n", "\n");
        if let Err(error) =
            self.checked(&["bash", "-c", &script, "stop-wsl-fixture", &self.directory])
        {
            eprintln!("Could not clean up WSL fixture: {error}");
        }
        if let Some(session) = &mut self.session {
            let _ = session.kill();
            let _ = session.wait();
        }
    }
}
