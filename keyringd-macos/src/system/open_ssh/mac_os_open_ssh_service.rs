use std::path::PathBuf;

use keyringd_core::service::{OpenSshService, SshConnectionTestResult};

pub(crate) struct MacOsOpenSshService {
    home: PathBuf,
    socket_path: PathBuf,
}

impl MacOsOpenSshService {
    pub(crate) fn new(home: PathBuf, socket_path: PathBuf) -> Self {
        Self { home, socket_path }
    }
    fn agent_configuration(&self) -> super::MacOsSshAgentConfiguration {
        super::MacOsSshAgentConfiguration::new(self.home.clone(), self.socket_path.clone())
    }
}

impl OpenSshService for MacOsOpenSshService {
    type Error = std::io::Error;

    fn configure(&self) -> Result<(), Self::Error> {
        self.agent_configuration().configure()
    }

    fn unconfigure(&self) -> Result<(), Self::Error> {
        self.agent_configuration().unconfigure()
    }
    fn has_managed_agent_configuration(&self) -> Result<bool, Self::Error> {
        self.agent_configuration().has_managed_agent_configuration()
    }
    async fn test_connection(
        &self,
        host: String,
        public_identity: String,
    ) -> SshConnectionTestResult {
        super::test_open_ssh_connection(
            host,
            self.socket_path.display().to_string(),
            public_identity,
        )
        .await
    }
}
