mod finder_launch;
mod handler;
mod mac_os_desktop;
mod mac_os_entry_point;
mod ui;
#[cfg(feature = "test-support")]
pub(crate) mod ui_test_support;

pub(crate) use mac_os_entry_point::MacOsEntryPoint;

mod native_mac_os_passkey_backend;
pub(crate) use native_mac_os_passkey_backend::NativeMacOsPasskeyBackend;

#[cfg(not(feature = "test-support"))]
mod mac_os_passkey_backend;
#[cfg(not(feature = "test-support"))]
pub(crate) use mac_os_passkey_backend::MacOsPasskeyBackend;

#[cfg(not(feature = "test-support"))]
mod mac_os_device_key_backend;
#[cfg(not(feature = "test-support"))]
pub(crate) use mac_os_device_key_backend::MacOsDeviceKeyBackend;

mod mac_os_application;
pub(crate) use mac_os_application::MacOsApplication;

#[cfg(feature = "test-support")]
pub(crate) use crate::app::ui_test_support::{MacOsDeviceKeyBackend, MacOsPasskeyBackend};

mod standalone_key_operation_prompts;
pub(crate) use standalone_key_operation_prompts::StandaloneKeyOperationPrompts;

mod authentication_prompt_command_handler;
pub(crate) use authentication_prompt_command_handler::AuthenticationPromptCommandHandler;

mod mac_os_key_operation_session;
pub(crate) use mac_os_key_operation_session::MacOsKeyOperationSession;
mod ssh_agent_selection;
mod ssh_agent_selection_service;
pub(crate) use ssh_agent_selection_service::SshAgentSelectionService;
