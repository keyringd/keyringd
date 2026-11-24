use keyringd_core::service::{OpenSshService, SshConnectionTestResult};
use std::sync::Mutex;

#[derive(Default)]
pub(crate) struct RecordingOpenSshService {
    pub(crate) requests: Mutex<Vec<(String, String)>>,
}
impl OpenSshService for RecordingOpenSshService {
    type Error = std::io::Error;
    fn configure(&self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn unconfigure(&self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn has_managed_agent_configuration(&self) -> Result<bool, Self::Error> {
        Ok(true)
    }
    async fn test_connection(
        &self,
        host: String,
        public_identity: String,
    ) -> SshConnectionTestResult {
        self.requests.lock().unwrap().push((host, public_identity));
        SshConnectionTestResult::IdentityRejected
    }
}
