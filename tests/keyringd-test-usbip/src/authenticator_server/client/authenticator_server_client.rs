use crate::authenticator_server::client::{
    AuthenticatorConfiguration, AuthenticatorHandle, AuthenticatorInfo, AuthenticatorServerError,
};
use std::net::SocketAddr;
use std::time::Duration;

/// HTTP control client for the shared Kotlin authenticator server.
#[derive(Clone, Debug)]
pub struct AuthenticatorServerClient {
    address: SocketAddr,
    timeout: Duration,
}
impl AuthenticatorServerClient {
    pub fn new(address: SocketAddr) -> Self {
        Self {
            address,
            timeout: Duration::from_secs(5),
        }
    }

    pub fn from_endpoint(endpoint: &str) -> Result<Self, AuthenticatorServerError> {
        let address = endpoint
            .strip_prefix("http://")
            .ok_or(AuthenticatorServerError::InvalidEndpoint)?
            .trim_end_matches('/')
            .parse()
            .map_err(|_| AuthenticatorServerError::InvalidEndpoint)?;
        Ok(Self::new(address))
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn authenticator(
        &self,
        id: impl Into<String>,
    ) -> Result<AuthenticatorHandle, AuthenticatorServerError> {
        let id = id.into();
        if !id.bytes().all(|b| b.is_ascii_digit()) || !id.parse::<u16>().is_ok_and(|id| id > 0) {
            return Err(AuthenticatorServerError::InvalidIdentifier);
        }
        Ok(AuthenticatorHandle::new(self.clone(), id))
    }

    pub async fn health(&self) -> Result<(), AuthenticatorServerError> {
        self.request("GET", "/health", &[]).await?;
        Ok(())
    }

    pub async fn authenticators(&self) -> Result<Vec<AuthenticatorInfo>, AuthenticatorServerError> {
        Ok(serde_json::from_slice(
            &self.request("GET", "/authenticators", &[]).await?,
        )?)
    }

    pub async fn create_authenticator(
        &self,
        configuration: AuthenticatorConfiguration,
    ) -> Result<AuthenticatorHandle, AuthenticatorServerError> {
        let info: AuthenticatorInfo = serde_json::from_slice(
            &self
                .request(
                    "POST",
                    "/authenticators",
                    &serde_json::to_vec(&configuration)?,
                )
                .await?,
        )?;
        self.authenticator(info.id)
    }

    pub(crate) async fn request(
        &self,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Result<Vec<u8>, AuthenticatorServerError> {
        super::request(self.address, self.timeout, method, path, body).await
    }
}
