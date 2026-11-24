mod test_app;
pub use test_app::TestApp;
pub mod diagnostics;

// Check the actual fixture CLI contract without AppKit.
#[cfg(all(test, target_os = "linux"))]
#[path = "../../../../../keyringd-macos/src/app/ui_test_support/fixture_arguments.rs"]
mod fixture_arguments;

#[cfg(target_os = "macos")]
mod macos_system_test_environment;
#[cfg(target_os = "macos")]
pub use macos_system_test_environment::MacosSystemTestEnvironment;

mod macos_system_test_application;
pub use macos_system_test_application::MacosSystemTestApplication;
