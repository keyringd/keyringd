use super::SshTestServer;
use anyhow::Result;

pub trait SshServers {
    type Server: SshTestServer;
    fn start(&self, trust: &str) -> Result<Self::Server>;
}
