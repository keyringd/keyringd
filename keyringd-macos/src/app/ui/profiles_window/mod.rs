#[allow(clippy::module_inception)]
mod profiles_window;
mod profiles_window_command_handler;

pub(crate) use keyringd_ui_core::profiles_window::ProfilesWindowCommand;
pub(crate) use keyringd_ui_core::profiles_window::ProfilesWindowEvent;
pub(crate) use keyringd_ui_core::profiles_window::ProfilesWindowState;
pub(crate) use profiles_window::ProfilesWindow;
pub(crate) use profiles_window_command_handler::ProfilesWindowCommandHandler;
