use super::{SshServers, UnixSshServer};
use anyhow::Result;
use std::{path::PathBuf, sync::Arc};
use tokio::runtime::Runtime;
pub struct UnixSshServers {
    runtime: Arc<Runtime>,
    parent: Option<PathBuf>,
}
impl UnixSshServers {
    pub(crate) fn new(runtime: Arc<Runtime>, parent: Option<PathBuf>) -> Self {
        Self { runtime, parent }
    }
}
impl SshServers for UnixSshServers {
    type Server = UnixSshServer;
    fn start(&self, trust: &str) -> Result<Self::Server> {
        UnixSshServer::start(Arc::clone(&self.runtime), self.parent.as_deref(), trust)
    }
}
