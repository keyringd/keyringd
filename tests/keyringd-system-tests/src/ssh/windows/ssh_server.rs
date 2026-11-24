//! Same-user sshd with a dedicated configuration for each test.
use super::WindowsSshSession;
use super::process_job::ProcessJob;
use crate::ssh::SshClient;
use anyhow::{Context, Result, ensure};
use keyringd_testkit::ssh::SshCommandResult;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct SshServer {
    directory: tempfile::TempDir,
    binaries: PathBuf,
    server: Child,
    job: ProcessJob,
    log_reader: Option<std::thread::JoinHandle<()>>,
    log: PathBuf,
}
impl SshServer {
    pub fn start(trust_entry: &str) -> Result<Self> {
        let binaries = PathBuf::from(
            std::env::var_os("KEYRINGD_TEST_OPENSSH")
                .context("run cargo windows-system-test to configure the OpenSSH fixture")?,
        );
        let directory = tempfile::tempdir()?;
        std::fs::write(
            directory.path().join("authorized_keys"),
            format!("{trust_entry}\n"),
        )?;
        let pipe = super::CurrentUser::agent_pipe_name()?;
        super::prepare_sshd::prepare(directory.path(), &binaries, &pipe)
            .context("sshd setup failed")?;
        let job = ProcessJob::new()?;
        let mut server = Command::new(binaries.join("sshd.exe"))
            .creation_flags(CREATE_NO_WINDOW)
            .args(["-D", "-e", "-f"])
            .arg(directory.path().join("sshd_config"))
            .current_dir(&binaries)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        // Attach before any client connects, so sshd-session and shell children
        // are owned by this case even when an assertion panics.
        if let Err(error) = job.assign(&server) {
            let _ = server.kill();
            let _ = server.wait();
            return Err(error);
        }
        let stderr = server.stderr.take().context("missing sshd log stream")?;
        let log = directory.path().join("sshd.log");
        let mut log_file = File::create(&log)?;
        let (ready, readiness) = mpsc::channel();
        let log_reader = std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let Ok(line) = line else {
                    break;
                };
                let _ = writeln!(log_file, "{line}");
                if line.contains("Server listening on ") {
                    let _ = ready.send(());
                }
            }
        });
        let mut result = Self {
            directory,
            binaries,
            server,
            job,
            log_reader: Some(log_reader),
            log,
        };
        // Wait for the server's own readiness log instead of probing the port.
        if let Err(error) = readiness.recv_timeout(Duration::from_secs(5)) {
            let cleanup = result.stop();
            anyhow::bail!(
                "sshd did not listen: {error}: {}; cleanup: {cleanup:?}",
                result.server_log()?
            );
        }
        Ok(result)
    }
    pub fn set_client_agent(&self, agent: &str) -> Result<()> {
        let agent = agent.replace('\\', "/");
        let path = self.directory.path().join("ssh_config");
        let config = std::fs::read_to_string(&path)?;
        let config = config
            .lines()
            .map(|line| {
                if line.trim_start().starts_with("IdentityAgent ") {
                    format!("    IdentityAgent {agent}")
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(path, config)?;
        Ok(())
    }
    fn command(
        &self,
        executable: &std::path::Path,
        identity: &str,
        agent_environment: Option<&str>,
        remote_command: &str,
    ) -> Result<Command> {
        let path = self.directory.path().join("identity.pub");
        std::fs::write(&path, format!("{identity}\n"))?;
        let mut command = Command::new(executable);
        command
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .arg("-F")
            .arg(self.directory.path().join("ssh_config"))
            .args([
                "-v",
                "-T",
                "-o",
                "IdentitiesOnly=yes",
                "-o",
                "PreferredAuthentications=publickey",
                "-i",
            ])
            .arg(path);
        if let Some(agent) = agent_environment {
            command
                .env("SSH_AUTH_SOCK", agent)
                .args(["-o", "IdentityAgent=SSH_AUTH_SOCK"]);
        }
        command.args(["--", "ui-test-host", remote_command]);
        Ok(command)
    }
    pub fn spawn_client(&self, identity: &str, command: &str) -> Result<SshClient> {
        SshClient::new(
            self.command(&self.binaries.join("ssh.exe"), identity, None, command)?
                .spawn()?,
        )
    }
    pub fn connect_with_agent_environment(
        &mut self,
        identity: &str,
        agent: &str,
    ) -> Result<SshCommandResult> {
        let client = SshClient::new(
            self.command(
                &self.binaries.join("ssh.exe"),
                identity,
                Some(agent),
                "echo keyringd-ssh-ok",
            )?
            .spawn()?,
        )?;
        WindowsSshSession::finish_client(client, self)
    }
    pub fn connect(&mut self, identity: &str) -> Result<SshCommandResult> {
        let client = self.spawn_client(identity, "echo keyringd-ssh-ok")?;
        WindowsSshSession::finish_client(client, self)
    }
    pub fn identity_file(&self) -> PathBuf {
        self.directory.path().join("identity.pub")
    }

    pub fn connect_with_git_for_windows(
        &mut self,
        identity: &str,
        socket: &std::path::Path,
    ) -> Result<SshCommandResult> {
        let binaries =
            PathBuf::from(std::env::var_os("ProgramFiles").context("ProgramFiles is not set")?)
                .join("Git/usr/bin");
        ensure!(
            binaries.join("ssh.exe").is_file(),
            "Git for Windows SSH is required"
        );
        // Keep the dedicated config, selected public identity and bounded
        // execution, but use Git's actual MSYS-based SSH executable.
        let mut command = self.command(
            &binaries.join("ssh.exe"),
            identity,
            Some(&socket.to_string_lossy()),
            "echo keyringd-ssh-ok",
        )?;
        WindowsSshSession::finish_client(SshClient::new(command.spawn()?)?, self)
    }
    pub fn stop(&mut self) -> Result<()> {
        self.job.terminate();
        if self.server.try_wait()?.is_none() {
            self.server.kill()?;
        }
        self.server.wait()?;
        if let Some(reader) = self.log_reader.take() {
            reader
                .join()
                .map_err(|_| anyhow::anyhow!("sshd log reader panicked"))?;
        }
        Ok(())
    }
    pub fn server_log(&self) -> Result<String> {
        Ok(std::fs::read_to_string(&self.log)?)
    }
}
impl Drop for SshServer {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("failed to stop the case SSH server: {error:#}");
        }
    }
}

impl crate::ssh::SshTestServer for SshServer {
    type Session = super::WindowsSshSession;
    fn start_client(
        self,
        agent: &str,
        identity: &keyringd_testkit::agent::AgentIdentity,
        command: &str,
    ) -> Result<Self::Session> {
        self.set_client_agent(agent)?;
        let client = SshServer::spawn_client(&self, &identity.to_openssh(), command)?;
        let selected = keyringd_testkit::agent::AgentIdentity::from_openssh(
            std::fs::read_to_string(self.identity_file())?.trim(),
        )?;
        ensure!(
            selected == *identity,
            "SSH IdentityFile differs from selected Agent identity"
        );
        Ok(super::WindowsSshSession::new(self, client))
    }

    fn client_configuration(&self) -> Result<crate::ssh::SshClientConfiguration> {
        let config_file = self.directory.path().join("ssh_config");
        let text = std::fs::read_to_string(&config_file)?;
        let field = |name: &str| -> Result<String> {
            text.lines()
                .find_map(|line| line.trim().strip_prefix(name).map(str::trim))
                .map(str::to_owned)
                .with_context(|| format!("SSH configuration has no {name}"))
        };
        Ok(crate::ssh::SshClientConfiguration {
            host: field("HostName ")?,
            port: field("Port ")?.parse()?,
            user: field("User ")?,
            use_agent: true,
            config_file,
        })
    }
    fn log(&self) -> Result<String> {
        self.server_log()
    }
    fn stop(&mut self) -> Result<()> {
        SshServer::stop(self)
    }
}
