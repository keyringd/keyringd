use super::SshServer;
use crate::ssh::SshServers;
use anyhow::Result;
pub struct WindowsSshServers;
impl SshServers for WindowsSshServers {
    type Server = SshServer;
    fn start(&self, trust: &str) -> Result<SshServer> {
        SshServer::start(trust)
    }
}
