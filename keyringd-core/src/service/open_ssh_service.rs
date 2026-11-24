use super::SshConnectionTestResult;

pub trait OpenSshService {
    type Error: std::error::Error + Send + Sync + 'static;

    fn configure(&self) -> Result<(), Self::Error>;

    fn unconfigure(&self) -> Result<(), Self::Error>;

    /// Returns whether Keyringd can remove its managed SSH agent configuration.
    fn has_managed_agent_configuration(&self) -> Result<bool, Self::Error>;

    fn test_connection(
        &self,
        host: String,
        public_identity: String,
    ) -> impl std::future::Future<Output = SshConnectionTestResult> + Send;
}
