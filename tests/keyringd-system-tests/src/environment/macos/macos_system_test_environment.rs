use super::MacosSystemTestApplication;
use crate::authenticator::Authenticators;
use crate::environment::SystemTestEnvironment;
use crate::ssh::UnixSshServers;
use anyhow::Result;
use std::sync::Arc;
pub struct MacosSystemTestEnvironment {
    application: MacosSystemTestApplication,
    authenticators: Authenticators,
    ssh_servers: UnixSshServers,
}
impl SystemTestEnvironment for MacosSystemTestEnvironment {
    type Application = MacosSystemTestApplication;
    type SshServers = UnixSshServers;
    fn can_disable_bluetooth() -> bool {
        true
    }
    fn new() -> Result<Self> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
        );
        let ssh_servers = UnixSshServers::new(Arc::clone(&runtime), None);
        let authenticators = Authenticators::from_environment(Arc::clone(&runtime))?;
        let application =
            MacosSystemTestApplication::new(Arc::clone(&runtime), authenticators.usb_address()?)?;
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
