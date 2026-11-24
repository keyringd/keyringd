use super::Environment;
use crate::environment::SocketActivationDriver;
use anyhow::{Result, ensure};
use std::path::PathBuf;
use tokio::runtime::Runtime;

/// Uses only the installed product service in the runner's isolated container.
pub struct SystemdAgentFixture {
    runtime: Runtime,
    environment: Environment,
}

impl SystemdAgentFixture {
    pub(crate) fn for_application() -> Result<Self> {
        Self::start(false)
    }
    fn start(reset: bool) -> Result<Self> {
        let fixture = Self {
            runtime: Runtime::new()?,
            environment: Environment::from_environment()?,
        };
        fixture.runtime.block_on(async {
            if reset {
                fixture.environment.reset_installed_application().await?;
            } else {
                fixture.environment.stop_installed_application().await?;
            }
            let started = fixture
                .environment
                .systemctl_user(&["start", "keyringd-agent.socket"])
                .await?;
            ensure!(
                started.status.success(),
                "starting socket failed: {}",
                started.stderr
            );
            Ok::<_, anyhow::Error>(())
        })?;
        Ok(fixture)
    }
}

impl SocketActivationDriver for SystemdAgentFixture {
    fn prepare() -> Result<Self> {
        Self::start(true)
    }

    fn socket_path(&self) -> Result<PathBuf> {
        Ok(self.environment.agent_socket().to_owned())
    }

    fn process_id(&self) -> Result<Option<u32>> {
        let output = self.runtime.block_on(self.environment.systemctl_user(&[
            "show",
            "--property=MainPID",
            "--value",
            "keyringd-desktop.service",
        ]))?;
        ensure!(
            output.status.success(),
            "reading MainPID failed: {}",
            output.stderr
        );
        let pid = output.stdout.trim().parse::<u32>()?;
        Ok((pid != 0).then_some(pid))
    }
}

impl Drop for SystemdAgentFixture {
    fn drop(&mut self) {
        if let Err(error) = self
            .runtime
            .block_on(self.environment.stop_installed_application())
        {
            eprintln!("Failed to stop the case-owned application and socket: {error:#}");
        }
    }
}
