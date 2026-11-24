use crate::authenticator_server::manager::{Endpoints, OwnedProcess, wait_ready};
use crate::{
    authenticator_server::client::AuthenticatorServerClient,
    authenticator_server::manager::{AuthenticatorServerOptions, AuthenticatorServerProcessError},
};
use std::{net::SocketAddr, path::Path};

/// Manages only the Kotlin AuthenticatorServer process it launched. A client
/// connected to an external server never acquires ownership of that process.
pub struct AuthenticatorServerManager {
    process: OwnedProcess,
    endpoints: Endpoints,
    client: AuthenticatorServerClient,
}
impl AuthenticatorServerManager {
    pub async fn start(
        options: AuthenticatorServerOptions,
    ) -> Result<Self, AuthenticatorServerProcessError> {
        validate(&options)?;
        let (endpoints, reservations) = Endpoints::reserve(&options)?;
        // Java binds its own sockets. The temporary reservations cannot be
        // inherited by it; a competing bind is reported as a startup failure.
        drop(reservations);
        let process = OwnedProcess::spawn(&options, &endpoints)?;
        let client = AuthenticatorServerClient::new(connect_address(endpoints.control));
        let mut manager = Self {
            process,
            endpoints,
            client,
        };
        wait_ready(
            &mut manager.process,
            &manager.endpoints,
            options.startup_timeout,
        )
        .await?;
        Ok(manager)
    }
    pub fn client(&self) -> &AuthenticatorServerClient {
        &self.client
    }
    pub fn usb_address(&self) -> SocketAddr {
        connect_address(self.endpoints.usb)
    }
    pub fn control_address(&self) -> SocketAddr {
        connect_address(self.endpoints.control)
    }
    pub fn hybrid_address(&self) -> Option<SocketAddr> {
        self.endpoints.hybrid.map(connect_address)
    }
    pub fn process_id(&self) -> Option<u32> {
        self.process.id()
    }
    /// Stop and reap. Repeating a successful stop is harmless. On failure the
    /// process remains owned so Drop can retry cleanup.
    pub fn stop(&mut self) -> Result<(), AuthenticatorServerProcessError> {
        self.process.stop()
    }
}
use crate::authenticator_server::manager::connect_address;
fn validate(options: &AuthenticatorServerOptions) -> Result<(), AuthenticatorServerProcessError> {
    let invalid = |message: &str| AuthenticatorServerProcessError::InvalidOptions(message.into());
    if options.startup_timeout.is_zero() {
        return Err(invalid("startup timeout must be positive"));
    }
    if !options.jar.is_file() {
        return Err(invalid("JAR must be an existing file"));
    }
    if options.log_path.as_os_str().is_empty() {
        return Err(invalid("log path is required"));
    }
    if let Some(hybrid) = &options.hybrid
        && (!hybrid.certificate_chain_pem.is_file() || !hybrid.private_key_pem.is_file())
    {
        return Err(invalid(
            "Hybrid requires an existing PEM certificate chain and private key",
        ));
    }
    // Validate before truncating the log: it must never replace an input file.
    let same_file = |path: &Path| {
        std::fs::canonicalize(&options.log_path)
            .ok()
            .zip(std::fs::canonicalize(path).ok())
            .is_some_and(|(a, b)| a == b)
    };
    if same_file(&options.jar)
        || options
            .hybrid
            .as_ref()
            .is_some_and(|h| same_file(&h.certificate_chain_pem) || same_file(&h.private_key_pem))
    {
        return Err(invalid("log path must not overwrite JAR or TLS material"));
    }
    Ok(())
}
