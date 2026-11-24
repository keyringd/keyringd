mod application_load_failure;
pub mod handler;
mod startup_failure_window;
mod startup_mode;
pub(crate) mod ui;
#[cfg(feature = "test-support")]
pub(crate) mod ui_test_support;
mod windows_desktop;
mod windows_entry_point;

mod windows_error;

pub(crate) use application_load_failure::ApplicationLoadFailure;
pub(crate) use startup_mode::StartupMode;
pub(crate) use windows_desktop::WindowsDesktop;

mod native_windows_passkey_backend;
pub(crate) use native_windows_passkey_backend::NativeWindowsPasskeyBackend;

#[cfg(not(feature = "test-support"))]
mod windows_passkey_backend;
#[cfg(not(feature = "test-support"))]
pub(crate) use windows_passkey_backend::WindowsPasskeyBackend;

#[cfg(not(feature = "test-support"))]
mod windows_device_key_backend;
#[cfg(not(feature = "test-support"))]
pub(crate) use windows_device_key_backend::WindowsDeviceKeyBackend;

mod windows_application;
pub(crate) use windows_application::WindowsApplication;

#[cfg(feature = "test-support")]
pub(crate) use ui_test_support::{WindowsDeviceKeyBackend, WindowsPasskeyBackend};

pub use windows_entry_point::WindowsEntryPoint;

pub(crate) use windows_error::WindowsError;

mod standalone_key_operation_prompts;
pub(crate) use standalone_key_operation_prompts::StandaloneKeyOperationPrompts;

mod authentication_prompt_windows;
pub(crate) use authentication_prompt_windows::AuthenticationPromptWindows;

mod authentication_prompt_command_handler;
pub(crate) use authentication_prompt_command_handler::AuthenticationPromptCommandHandler;
