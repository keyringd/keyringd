use super::{Environment, LinuxTestbedGuard};
use anyhow::Result;
use std::sync::Arc;
use tokio::runtime::Runtime;
pub struct LinuxTestbed {
    environment: Environment,
    runtime: Arc<Runtime>,
}
impl LinuxTestbed {
    pub(crate) fn new(environment: Environment, runtime: Arc<Runtime>) -> Self {
        Self {
            environment,
            runtime,
        }
    }
    pub fn ssh_client_configuration_text(&self) -> Result<String> {
        Ok(std::fs::read_to_string(
            self.ssh_client_configuration_path(),
        )?)
    }
    pub fn ssh_client_configuration_exists(&self) -> Result<bool> {
        Ok(self.ssh_client_configuration_path().try_exists()?)
    }
    fn ssh_client_configuration_path(&self) -> std::path::PathBuf {
        self.environment
            .root()
            .join("config/environment.d/60-keyringd.conf")
    }
    pub fn generated_ssh_agent_environment(&self) -> Result<super::CommandOutput> {
        let output = self.runtime.block_on(async {
            self.environment.command("/usr/lib/systemd/user-environment-generators/30-systemd-environment-d-generator")
                .env("XDG_CONFIG_HOME", self.environment.root().join("config"))
                .env_remove("SSH_AUTH_SOCK").output().await
        })?;
        Ok(output.into())
    }
    pub fn assert_application_is_isolated(&self) -> Result<()> {
        self.runtime
            .block_on(self.environment.assert_installed_application_is_isolated())
    }
    pub fn systemctl_user(&self, arguments: &[&str]) -> Result<super::CommandOutput> {
        self.runtime
            .block_on(self.environment.systemctl_user(arguments))
    }
    pub fn agent_service_active_state(&self) -> Result<String> {
        self.runtime
            .block_on(self.environment.agent_service_active_state())
    }
    pub fn agent_journal(&self) -> Result<String> {
        self.runtime.block_on(self.environment.agent_journal())
    }
    pub fn ble_advertiser(&self) -> Result<super::VirtualBleAdvertiser> {
        super::VirtualBleAdvertiser::from_environment(Arc::clone(&self.runtime))
    }
    pub fn disable_bluetooth(&self) -> Result<LinuxTestbedGuard> {
        LinuxTestbedGuard::disabled(self.environment.clone(), Arc::clone(&self.runtime))
    }
    pub fn hide_bluetooth_adapters(&self) -> Result<LinuxTestbedGuard> {
        LinuxTestbedGuard::hidden(self.environment.clone(), Arc::clone(&self.runtime))
    }
    pub fn remote_hybrid_advertisements(&self) -> Result<LinuxTestbedGuard> {
        LinuxTestbedGuard::remote(self.environment.clone(), Arc::clone(&self.runtime))
    }
}
