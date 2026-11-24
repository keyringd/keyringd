#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(any(
    target_os = "macos",
    all(target_os = "linux", feature = "test-support")
))]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

mod system_test_environment;
#[cfg(target_os = "linux")]
pub use linux::LinuxSystemTestEnvironment as NativeSystemTestEnvironment;
#[cfg(target_os = "macos")]
pub use macos::MacosSystemTestEnvironment as NativeSystemTestEnvironment;
pub use system_test_environment::SystemTestEnvironment;
#[cfg(target_os = "windows")]
pub use windows::WindowsSystemTestEnvironment as NativeSystemTestEnvironment;

#[cfg(target_os = "linux")]
mod socket_activation_driver;
#[cfg(target_os = "linux")]
pub use linux::SystemdAgentFixture as NativeSocketActivationFixture;
#[cfg(target_os = "linux")]
pub use socket_activation_driver::SocketActivationDriver;
