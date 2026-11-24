use keyringd_core::service::SshConnectionTestResult;
use std::sync::Mutex;

pub(crate) struct ControlledOpenSsh {
    configuration: Mutex<Option<(String, String)>>,
    requests: Mutex<Vec<serde_json::Value>>,
}

impl ControlledOpenSsh {
    pub(crate) fn new() -> Self {
        Self {
            configuration: Mutex::new(None),
            requests: Mutex::new(Vec::new()),
        }
    }
    pub(crate) fn set_configuration(&self, config: &str, pipe: &str) {
        *self.configuration.lock().unwrap() = Some((config.to_owned(), pipe.to_owned()));
    }
    pub(crate) fn requests(&self) -> Vec<serde_json::Value> {
        self.requests.lock().unwrap().clone()
    }
    pub(crate) async fn test_connection(
        &self,
        host: String,
        public_identity: String,
    ) -> SshConnectionTestResult {
        self.requests
            .lock()
            .unwrap()
            .push(serde_json::json!({"host": host, "public_identity": public_identity}));
        let configuration = self.configuration.lock().unwrap().clone();
        if let Some((config, pipe)) = configuration {
            return crate::system::ssh_connection_test::test_configured(
                host,
                public_identity,
                config,
                pipe,
            )
            .await;
        }
        SshConnectionTestResult::ClientConfigurationProblem(
            "UI test OpenSSH configuration is missing.".into(),
        )
    }
}

impl keyringd_core::service::OpenSshService for ControlledOpenSsh {
    type Error = std::io::Error;
    fn configure(&self) -> Result<(), Self::Error> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
    fn unconfigure(&self) -> Result<(), Self::Error> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
    fn has_managed_agent_configuration(&self) -> Result<bool, Self::Error> {
        Ok(self.configuration.lock().unwrap().is_some())
    }
    async fn test_connection(
        &self,
        host: String,
        public_identity: String,
    ) -> SshConnectionTestResult {
        ControlledOpenSsh::test_connection(self, host, public_identity).await
    }
}
