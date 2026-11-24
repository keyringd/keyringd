//! An isolated sshd for integration tests.
//!
//! Every instance owns its own temporary directory with a host key, config and
//! authorized_keys file. Nothing outside the directory is touched — in
//! particular the user's `~/.ssh` is never read or written.

use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use std::time::Duration;

use tokio::process::{Child, Command};

/// An sshd process and the artifacts it needs.
///
/// The process is killed and the temporary directory removed when this value
/// drops.
pub struct TestSshd {
    _child: Child,
    port: u16,
    log_path: PathBuf,
    _directory: tempfile::TempDir,
}

use super::SshOutput;

impl TestSshd {
    /// Starts an sshd on a random port that accepts only the given key
    /// material. `authorized_keys` is whatever a Trust Entry copy produced
    /// (e.g. `cert-authority,principals=… sk-ecdsa-…`).
    pub async fn start(authorized_keys: &str) -> anyhow::Result<Self> {
        Self::start_in(None, authorized_keys).await
    }

    /// Like [`start`](Self::start) but places the working directory under
    /// `parent` and writes the sshd log to `log_path`.
    pub async fn start_in(parent: Option<&Path>, authorized_keys: &str) -> anyhow::Result<Self> {
        use std::os::unix::fs::PermissionsExt;

        let mut builder = tempfile::Builder::new();
        builder.prefix("p4s-sshd-");
        let directory = if let Some(parent) = parent {
            builder.tempdir_in(parent)?
        } else {
            builder.tempdir()?
        };
        let host_key = directory.path().join("host_key");
        let output = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&host_key)
            .output()
            .await?;
        anyhow::ensure!(
            output.status.success(),
            "ssh-keygen failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let authorized_keys_path = directory.path().join("authorized_keys");
        std::fs::write(&authorized_keys_path, format!("{authorized_keys}\n"))?;
        std::fs::set_permissions(
            &authorized_keys_path,
            std::fs::Permissions::from_mode(0o600),
        )?;

        let port = TcpListener::bind(("127.0.0.1", 0))?.local_addr()?.port();
        let log_path = directory.path().join("sshd.log");
        let config = format!(
            "Port {port}\n\
             ListenAddress 127.0.0.1\n\
             HostKey {}\n\
             PubkeyAuthentication yes\n\
             AuthorizedKeysFile {}\n\
             PasswordAuthentication no\n\
             KbdInteractiveAuthentication no\n\
             UsePAM no\n\
             StrictModes yes\n\
             PidFile {}\n\
             LogLevel VERBOSE\n",
            host_key.display(),
            authorized_keys_path.display(),
            directory.path().join("sshd.pid").display(),
        );
        let config_path = directory.path().join("sshd_config");
        std::fs::write(&config_path, config)?;
        let log = std::fs::File::create(&log_path)?;
        let child = Command::new("/usr/sbin/sshd")
            .arg("-D")
            .arg("-e")
            .arg("-f")
            .arg(&config_path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(log)
            .kill_on_drop(true)
            .spawn()?;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                return Ok(Self {
                    _child: child,
                    port,
                    log_path,
                    _directory: directory,
                });
            }
            anyhow::ensure!(
                tokio::time::Instant::now() < deadline,
                "sshd did not listen on port {port}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Stop and reap the owned daemon before its files or runtime are released.
    pub async fn stop(&mut self) -> anyhow::Result<()> {
        if self._child.try_wait()?.is_none() {
            self._child.start_kill()?;
        }
        self._child.wait().await?;
        Ok(())
    }

    /// The port sshd listens on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The sshd log since it started.
    pub fn log(&self) -> String {
        std::fs::read_to_string(&self.log_path).unwrap_or_default()
    }

    /// Starts a client owned and reaped by the caller, with diagnostic pipes.
    pub fn start_client(
        &self,
        agent_socket: &Path,
        identity_file: Option<&Path>,
        command: &str,
    ) -> anyhow::Result<std::process::Child> {
        let mut client = self.ssh_command();
        if let Some(path) = identity_file {
            client.args(["-o", "IdentitiesOnly=yes", "-i"]).arg(path);
        }
        Ok(client
            .as_std_mut()
            .args(["-F", "/dev/null"])
            .arg("-o")
            .arg(format!("IdentityAgent={}", agent_socket.display()))
            .arg("127.0.0.1")
            .arg(command)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?)
    }

    /// Runs `ssh` against this sshd with the given agent socket.
    pub async fn ssh(&self, agent_socket: &Path, command: &str) -> anyhow::Result<SshOutput> {
        let mut client = self.ssh_command();
        client
            .args(["-F", "/dev/null"])
            .arg("-o")
            .arg(format!("IdentityAgent={}", agent_socket.display()));
        self.execute_ssh(client, command).await
    }

    /// Offer only this public identity, so a negative test cannot succeed by
    /// silently choosing another profile or a certificate from the Agent.
    pub async fn ssh_with_identity_file(
        &self,
        agent_socket: &Path,
        identity_file: &Path,
        command: &str,
    ) -> anyhow::Result<SshOutput> {
        let mut client = self.ssh_command();
        client
            .args(["-F", "/dev/null", "-o", "IdentitiesOnly=yes", "-i"])
            .arg(identity_file)
            .arg("-o")
            .arg(format!("IdentityAgent={}", agent_socket.display()));
        self.execute_ssh(client, command).await
    }

    fn ssh_command(&self) -> Command {
        let mut client = Command::new("ssh");
        client
            .kill_on_drop(true)
            .arg("-v")
            .args(["-p", &self.port.to_string()])
            .args(["-o", "UserKnownHostsFile=/dev/null"])
            .args(["-o", "GlobalKnownHostsFile=/dev/null"])
            .args(["-o", "StrictHostKeyChecking=no"])
            .args(["-o", "BatchMode=yes"])
            // macOS OpenSSH otherwise discards -i Security Key identities before
            // matching them against the agent. The agent performs the signing;
            // the client never needs to load a local hardware provider.
            .args(["-o", "SecurityKeyProvider=internal"])
            .args(["-o", "PasswordAuthentication=no"])
            .args(["-o", "KbdInteractiveAuthentication=no"])
            .args(["-o", "ConnectTimeout=5"]);
        client
    }

    async fn execute_ssh(&self, mut client: Command, command: &str) -> anyhow::Result<SshOutput> {
        let output = tokio::time::timeout(
            Duration::from_secs(30),
            client.arg("127.0.0.1").arg(command).output(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("SSH authentication timed out"))??;
        Ok(output.into())
    }
}
