#[allow(clippy::module_inception)]
mod profile_setup_window;
pub(crate) use profile_setup_window::ProfileSetupWindow;

mod profile_setup_window_command_handler;
use profile_setup_window_command_handler::ProfileSetupWindowCommandHandler;
