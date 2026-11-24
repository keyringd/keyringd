use super::LinuxSystemTestApplication;
use crate::authenticator::Authenticators;
use crate::environment::SystemTestEnvironment;
use crate::ssh::UnixSshServers;
use anyhow::Result;
use std::sync::Arc;
pub struct LinuxSystemTestEnvironment {
    testbed: super::LinuxTestbed,
    application: LinuxSystemTestApplication,
    authenticators: Authenticators,
    ssh_servers: UnixSshServers,
}
impl SystemTestEnvironment for LinuxSystemTestEnvironment {
    type Application = LinuxSystemTestApplication;
    type SshServers = UnixSshServers;
    type ApplicationSocketActivation = super::SystemdAgentFixture;
    fn prepare_application_socket_activation(
        &mut self,
    ) -> Result<Self::ApplicationSocketActivation> {
        crate::application::SystemTestApplication::stop(&mut self.application)?;
        super::SystemdAgentFixture::for_application()
    }
    fn can_disable_bluetooth() -> bool {
        std::env::var_os("KEYRINGD_E2E_BLE_SCANNER").is_some()
    }
    fn new() -> Result<Self> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
        );
        let environment = super::Environment::from_environment()?;
        runtime.block_on(environment.reset_installed_application())?;
        let application =
            LinuxSystemTestApplication::new(environment.clone(), Arc::clone(&runtime))?;
        let ssh_servers =
            UnixSshServers::new(Arc::clone(&runtime), Some(environment.root().to_owned()));
        let authenticators = Authenticators::from_environment(Arc::clone(&runtime))?;
        Ok(Self {
            testbed: super::LinuxTestbed::new(environment, Arc::clone(&runtime)),
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

impl LinuxSystemTestEnvironment {
    pub fn testbed(&self) -> &super::LinuxTestbed {
        &self.testbed
    }
}
