#[cfg(any(
    target_os = "macos",
    all(target_os = "linux", feature = "test-support")
))]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

mod ssh_session;
pub use ssh_session::SshSession;
#[cfg(unix)]
mod unix_ssh_session;
#[cfg(unix)]
pub use unix_ssh_session::UnixSshSession;

mod ssh_client_configuration;
pub use ssh_client_configuration::SshClientConfiguration;
mod ssh_servers;
pub use ssh_servers::SshServers;
mod ssh_test_server;
pub use ssh_test_server::SshTestServer;

#[cfg(unix)]
mod unix_ssh_servers;
#[cfg(unix)]
pub use unix_ssh_servers::UnixSshServers;
#[cfg(unix)]
mod unix_ssh_server;
#[cfg(unix)]
pub use unix_ssh_server::UnixSshServer;

mod ssh_client;
pub use ssh_client::SshClient;
