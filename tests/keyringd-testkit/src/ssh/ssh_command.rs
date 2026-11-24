use std::path::Path;

use super::TestSshd;
use anyhow::Result;
use tokio::runtime::Runtime;

use super::SshCommandResult;

pub fn run_agent_ssh(
    runtime: &Runtime,
    root: Option<&Path>,
    agent: &Path,
    trust: &str,
    selected_identity: Option<&crate::agent::AgentIdentity>,
) -> Result<SshCommandResult> {
    runtime.block_on(async {
        let server = TestSshd::start_in(root, trust).await?;
        let directory = tempfile::tempdir()?;
        let identity_file = selected_identity.map(|_| directory.path().join("identity.pub"));
        let output = if let (Some(identity), Some(path)) = (selected_identity, &identity_file) {
            std::fs::write(path, format!("{}\n", identity.to_openssh()))?;
            server
                .ssh_with_identity_file(agent, path, "echo keyringd-ssh-ok")
                .await?
        } else {
            server.ssh(agent, "echo keyringd-ssh-ok").await?
        };
        Ok(SshCommandResult {
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
            server_log: server.log(),
            identity_file,
        })
    })
}
