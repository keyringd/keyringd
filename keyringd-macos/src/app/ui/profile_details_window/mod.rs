#[allow(clippy::module_inception)]
mod profile_details_window;
mod profile_details_window_action_target;
mod profile_details_window_command_handler;

pub(crate) use keyringd_ui_core::profile_details_window::{
    ProfileDetailsEvent, ProfileDetailsState,
};
pub(crate) use profile_details_window::ProfileDetailsWindow;
use profile_details_window_action_target::ProfileDetailsWindowActionTarget;
use profile_details_window_command_handler::ProfileDetailsWindowCommandHandler;
