use super::SshServer;
use crate::ssh::SshClient;
use crate::ssh::SshSession;
use anyhow::Result;
use keyringd_testkit::ssh::SshCommandResult;

pub struct WindowsSshSession {
    client: SshClient,
    server: SshServer,
}

impl WindowsSshSession {
    pub fn new(server: SshServer, client: SshClient) -> Self {
        Self { server, client }
    }

    pub(super) fn finish_client(
        client: SshClient,
        server: &mut SshServer,
    ) -> Result<SshCommandResult> {
        let output = client.finish()?;
        server.stop()?;
        Ok(SshCommandResult {
            status: output.status,
            stdout: String::from_utf8(output.stdout)?,
            stderr: String::from_utf8(output.stderr)?,
            server_log: server.server_log()?,
            identity_file: Some(server.identity_file()),
        })
    }
}

impl SshSession for WindowsSshSession {
    fn is_finished(&mut self) -> Result<bool> {
        self.client.is_finished()
    }

    fn finish(mut self) -> Result<SshCommandResult> {
        Self::finish_client(self.client, &mut self.server)
    }
}
