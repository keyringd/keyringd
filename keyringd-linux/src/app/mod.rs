mod adwaita_log_filter;
mod command_line;
pub(crate) mod handler;
mod linux_desktop;
mod linux_entry_point;
mod remote_hybrid_advertisement_relay;
pub(crate) mod service;
mod startup_failure_window;
mod startup_mode;
mod startup_mode_error;
mod ui;

use command_line::CommandLine;
use linux_desktop::LinuxDesktop;
pub(crate) use linux_entry_point::LinuxEntryPoint;
use startup_mode::StartupMode;
use startup_mode_error::StartupModeError;

mod linux_application;
pub(crate) use linux_application::LinuxApplication;

mod application_identity;
pub(crate) use application_identity::APPLICATION_ID;

mod standalone_key_operation_prompts;
pub(crate) use standalone_key_operation_prompts::StandaloneKeyOperationPrompts;

mod authentication_prompt_command_handler;
use authentication_prompt_command_handler::AuthenticationPromptCommandHandler;
