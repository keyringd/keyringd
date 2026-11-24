mod ssh_connection_identity;
pub use ssh_connection_identity::SshConnectionIdentity;
mod ssh_connection_request;
pub use ssh_connection_request::SshConnectionRequest;
mod ssh_connection_result;
pub use ssh_connection_result::{SshConnectionResult, SshRejectionGuidance};
mod ssh_connection_test_window_driver;
pub use ssh_connection_test_window_driver::SshConnectionTestWindowDriver;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(any(
    target_os = "macos",
    all(target_os = "linux", feature = "test-support")
))]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

mod connection_test_result_window_driver;
pub use connection_test_result_window_driver::ConnectionTestResultWindowDriver;
