#[allow(clippy::module_inception)]
mod profile_details_window;
pub(crate) use profile_details_window::ProfileDetailsWindow;
mod profile_details_command_handler;
use profile_details_command_handler::ProfileDetailsCommandHandler;
