use anyhow::Result;

pub trait SshTestServer: Sized {
    type Session: super::SshSession;
    fn start_client(
        self,
        agent: &str,
        identity: &keyringd_testkit::agent::AgentIdentity,
        command: &str,
    ) -> Result<Self::Session>;
    fn client_configuration(&self) -> Result<super::SshClientConfiguration>;
    fn log(&self) -> Result<String>;
    fn stop(&mut self) -> Result<()>;
}
