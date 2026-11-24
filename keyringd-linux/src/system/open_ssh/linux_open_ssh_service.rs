use std::path::PathBuf;

use keyringd_core::service::{OpenSshService, SshConnectionTestResult};

pub(crate) struct LinuxOpenSshService;

impl OpenSshService for LinuxOpenSshService {
    type Error = std::io::Error;

    fn configure(&self) -> Result<(), Self::Error> {
        super::configuration::configure(&config_directory()?)
    }

    fn unconfigure(&self) -> Result<(), Self::Error> {
        super::configuration::unconfigure(&config_directory()?)
    }

    fn has_managed_agent_configuration(&self) -> Result<bool, Self::Error> {
        super::configuration::has_managed_agent_configuration(&config_directory()?)
    }

    async fn test_connection(
        &self,
        host: String,
        public_identity: String,
    ) -> SshConnectionTestResult {
        super::test_ssh_connection(&host, &public_identity).await
    }
}

fn config_directory() -> std::io::Result<PathBuf> {
    if let Some(directory) = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        return Ok(directory);
    }
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(|home| PathBuf::from(home).join(".config"))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME is not set"))
}
