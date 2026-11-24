use keyringd_core::service::{OpenSshService, SshConnectionTestResult};

pub(crate) struct WindowsOpenSshService;

impl OpenSshService for WindowsOpenSshService {
    type Error = std::io::Error;

    fn configure(&self) -> Result<(), Self::Error> {
        super::windows_ssh_client_configuration::configure()
    }

    fn unconfigure(&self) -> Result<(), Self::Error> {
        super::windows_ssh_client_configuration::unconfigure()
    }

    fn has_managed_agent_configuration(&self) -> Result<bool, Self::Error> {
        super::windows_ssh_client_configuration::has_managed_agent_configuration()
    }

    async fn test_connection(
        &self,
        host: String,
        public_identity: String,
    ) -> SshConnectionTestResult {
        super::ssh_connection_test::test(host, public_identity).await
    }
}
