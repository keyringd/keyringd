#[allow(clippy::module_inception)]
mod remove_profile_window;
mod remove_profile_window_command_handler;

pub(crate) use keyringd_ui_core::remove_profile_window::RemoveProfileProps;
pub(crate) use remove_profile_window::RemoveProfileWindow;
use remove_profile_window_command_handler::RemoveProfileWindowCommandHandler;
