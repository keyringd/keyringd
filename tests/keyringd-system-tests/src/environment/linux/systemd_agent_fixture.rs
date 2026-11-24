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
            if !started.status.success() {
                let diagnostics = fixture.socket_start_diagnostics().await;
                anyhow::bail!(
                    "starting socket failed: {}\nSystemd diagnostics before cleanup:\n{diagnostics}",
                    started.stderr.trim_end()
                );
            }
            Ok::<_, anyhow::Error>(())
        })?;
        Ok(fixture)
    }

    async fn socket_start_diagnostics(&self) -> String {
        let mut sections = Vec::new();
        for (title, program, arguments) in [
            (
                "unit status",
                "systemctl",
                vec![
                    "--user",
                    "status",
                    "keyringd-agent.socket",
                    "keyringd-desktop.service",
                    "--no-pager",
                    "--full",
                ],
            ),
            (
                "unit properties",
                "systemctl",
                vec![
                    "--user",
                    "show",
                    "keyringd-agent.socket",
                    "keyringd-desktop.service",
                    "--property=Id,ActiveState,SubState,Result,Job,MainPID,Listen",
                ],
            ),
            (
                "pending jobs",
                "systemctl",
                vec!["--user", "list-jobs", "--no-pager", "--full"],
            ),
            (
                "unit journal",
                "journalctl",
                vec![
                    "--user",
                    "--unit=keyringd-agent.socket",
                    "--unit=keyringd-desktop.service",
                    "--lines=100",
                    "--no-pager",
                    "--output=short-precise",
                ],
            ),
            (
                "listening Unix sockets",
                "ss",
                vec!["--unix", "--listening", "--processes"],
            ),
        ] {
            let body = match self.environment.run(program, &arguments).await {
                Ok(output) => format!("exit={}\n{}{}", output.status, output.stdout, output.stderr),
                Err(error) => format!("diagnostic command failed: {error:#}"),
            };
            sections.push(format!("[{title}]\n{}", body.trim_end()));
        }

        let socket = self.environment.agent_socket();
        let socket_state = match std::fs::symlink_metadata(socket) {
            Ok(metadata) => format!(
                "path={}\nexists=true\nfile_type={:?}\nlength={}",
                socket.display(),
                metadata.file_type(),
                metadata.len()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                format!("path={}\nexists=false", socket.display())
            }
            Err(error) => format!("path={}\nmetadata failed: {error}", socket.display()),
        };
        sections.push(format!("[activation socket]\n{socket_state}"));
        sections.join("\n\n")
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
