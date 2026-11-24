mod test_app;
pub use test_app::TestApp;
pub mod command_output;
pub mod process_elevation;
pub mod process_security_diagnostics;
mod registry_sandbox;
mod test_process;
pub(crate) use registry_sandbox::RegistrySandbox;
pub(crate) use test_process::TestProcess;

mod windows_system_test_environment;
pub use windows_system_test_environment::WindowsSystemTestEnvironment;

mod windows_system_test_application;
pub use windows_system_test_application::WindowsSystemTestApplication;
