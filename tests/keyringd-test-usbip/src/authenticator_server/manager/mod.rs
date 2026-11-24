//! Owns a Kotlin server process and uses the HTTP client for health checks.
//!
//! Entry point: [`AuthenticatorServerManager::start`].

mod authenticator_server_manager;
mod authenticator_server_options;
mod authenticator_server_process_error;
mod endpoints;
mod hybrid_tunnel_options;
mod owned_process;
mod readiness;

pub use authenticator_server_manager::AuthenticatorServerManager;
pub use authenticator_server_options::AuthenticatorServerOptions;
pub use authenticator_server_process_error::AuthenticatorServerProcessError;
pub use hybrid_tunnel_options::HybridTunnelOptions;

pub(crate) use endpoints::{Endpoints, connect_address};
pub(crate) use owned_process::OwnedProcess;
pub(crate) use readiness::wait_ready;

mod process_io;
pub(crate) use process_io::io;
