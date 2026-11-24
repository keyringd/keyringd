use super::SshSession;
use super::{SshTestServer, UnixSshServer};
use anyhow::{Context, Result};
use keyringd_testkit::agent::AgentIdentity;
use keyringd_testkit::ssh::SshCommandResult;
use std::path::Path;

pub struct UnixSshSession {
    client: super::SshClient,
    server: UnixSshServer,
    identity_file: tempfile::NamedTempFile,
}

impl UnixSshSession {
    pub(super) fn start(
        server: UnixSshServer,
        agent: &Path,
        identity: &AgentIdentity,
        command: &str,
    ) -> Result<Self> {
        let identity_file = tempfile::NamedTempFile::new()?;
        std::fs::write(identity_file.path(), format!("{}\n", identity.to_openssh()))?;
        let client = server
            .server
            .as_ref()
            .context("SSH server is stopped")?
            .start_client(agent, Some(identity_file.path()), command)?;
        let client = super::SshClient::new(client)?;
        Ok(Self {
            client,
            server,
            identity_file,
        })
    }
}

impl SshSession for UnixSshSession {
    fn is_finished(&mut self) -> Result<bool> {
        self.client.is_finished()
    }

    fn finish(mut self) -> Result<SshCommandResult> {
        let output = self.client.finish()?;
        self.server.stop()?;
        Ok(SshCommandResult {
            status: output.status,
            stdout: String::from_utf8(output.stdout)?,
            stderr: String::from_utf8(output.stderr)?,
            server_log: self.server.log()?,
            identity_file: Some(self.identity_file.path().to_owned()),
        })
    }
}
