use super::WindowsSystemTestApplication;
use crate::authenticator::Authenticators;
use crate::environment::SystemTestEnvironment;
use crate::ssh::windows::WindowsSshServers;
use anyhow::Result;
use std::sync::Arc;
pub struct WindowsSystemTestEnvironment {
    application: WindowsSystemTestApplication,
    authenticators: Authenticators,
    ssh_servers: WindowsSshServers,
}
impl WindowsSystemTestEnvironment {
    /// Query the broker's actual token; scenario processes need not be elevated.
    pub fn security_automation_is_elevated() -> Result<bool> {
        use crate::authenticator::windows::security_automation::{
            SecurityAutomationAction, SecurityAutomationClient,
        };
        if std::env::var_os("KEYRINGD_SECURITY_AUTOMATION_ADDRESS").is_none()
            && std::env::var_os("KEYRINGD_SECURITY_AUTOMATION_TOKEN").is_none()
        {
            return Ok(false);
        }
        let state = SecurityAutomationClient::from_environment()?
            .call("", SecurityAutomationAction::IsElevated)?;
        state
            .as_bool()
            .ok_or_else(|| anyhow::anyhow!("invalid broker elevation response"))
    }
}
impl SystemTestEnvironment for WindowsSystemTestEnvironment {
    type Application = WindowsSystemTestApplication;
    type SshServers = WindowsSshServers;
    fn can_disable_bluetooth() -> bool {
        true
    }
    fn new() -> Result<Self> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
        );
        let application = WindowsSystemTestApplication::new()?;
        let ssh_servers = WindowsSshServers;
        let authenticators = Authenticators::from_environment(Arc::clone(&runtime))?;
        Ok(Self {
            application,
            authenticators,
            ssh_servers,
        })
    }
    fn application(&mut self) -> &mut Self::Application {
        &mut self.application
    }
    fn authenticators(&self) -> &Authenticators {
        &self.authenticators
    }
    fn ssh_servers(&self) -> &Self::SshServers {
        &self.ssh_servers
    }
}
