//! Prepares the server JAR using Gradle's incremental build and shared outputs.
//!
//! Entry point: [`AuthenticatorServerBuild::prepare`].

mod authenticator_server_build;
#[cfg(windows)]
mod batch_path;
pub use authenticator_server_build::AuthenticatorServerBuild;
