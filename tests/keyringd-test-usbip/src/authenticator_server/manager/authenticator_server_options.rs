use crate::authenticator_server::manager::HybridTunnelOptions;
use std::{net::SocketAddr, path::PathBuf, time::Duration};

/// Inputs for starting a Kotlin AuthenticatorServer. The caller supplies a JAR
/// prepared by `authenticator_server::build` and optional TLS material.
pub struct AuthenticatorServerOptions {
    pub java_executable: PathBuf,
    pub jar: PathBuf,
    pub log_path: PathBuf,
    /// Port 0 selects a free port before launching Java.
    pub usb_address: SocketAddr,
    pub control_address: SocketAddr,
    pub startup_timeout: Duration,
    pub hybrid: Option<HybridTunnelOptions>,
}

impl AuthenticatorServerOptions {
    pub fn new(jar: PathBuf, log_path: PathBuf) -> Self {
        Self {
            java_executable: super::super::java::executable(),
            jar,
            log_path,
            usb_address: "127.0.0.1:0".parse().unwrap(),
            control_address: "127.0.0.1:0".parse().unwrap(),
            startup_timeout: Duration::from_secs(30),
            hybrid: None,
        }
    }
}
