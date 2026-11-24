#[allow(clippy::module_inception)]
mod remove_profile_window;
pub(crate) use remove_profile_window::RemoveProfileWindow;
mod remove_profile_command_handler;
use remove_profile_command_handler::RemoveProfileCommandHandler;
