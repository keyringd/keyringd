use super::{SshClientConfiguration, SshTestServer};
use anyhow::Result;
use keyringd_testkit::ssh::TestSshd;
use std::{path::Path, sync::Arc};
use tokio::runtime::Runtime;
pub struct UnixSshServer {
    pub(super) server: Option<TestSshd>,
    log: String,
    port: u16,
    runtime: Arc<Runtime>,
}
impl UnixSshServer {
    pub(crate) fn start(runtime: Arc<Runtime>, parent: Option<&Path>, trust: &str) -> Result<Self> {
        let server = runtime.block_on(TestSshd::start_in(parent, trust))?;
        Ok(Self {
            port: server.port(),
            server: Some(server),
            log: String::new(),
            runtime,
        })
    }
}
impl SshTestServer for UnixSshServer {
    type Session = super::UnixSshSession;
    fn start_client(
        self,
        agent: &str,
        identity: &keyringd_testkit::agent::AgentIdentity,
        command: &str,
    ) -> Result<Self::Session> {
        super::UnixSshSession::start(self, Path::new(agent), identity, command)
    }

    fn client_configuration(&self) -> Result<SshClientConfiguration> {
        Ok(SshClientConfiguration {
            host: "127.0.0.1".into(),
            port: self.port,
            use_agent: true,
            user: std::env::var("USER").unwrap_or_else(|_| "e2e".into()),
        })
    }
    fn log(&self) -> Result<String> {
        Ok(self
            .server
            .as_ref()
            .map_or_else(|| self.log.clone(), TestSshd::log))
    }
    fn stop(&mut self) -> Result<()> {
        if let Some(server) = self.server.as_mut() {
            self.runtime.block_on(server.stop())?;
            self.log = server.log();
        }
        self.server.take();
        Ok(())
    }
}

impl Drop for UnixSshServer {
    fn drop(&mut self) {
        if let Err(error) = SshTestServer::stop(self) {
            eprintln!("failed to stop and reap the case SSH server: {error:#}");
        }
    }
}
