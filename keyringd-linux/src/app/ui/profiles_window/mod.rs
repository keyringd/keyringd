#[allow(clippy::module_inception)]
mod profiles_window;
pub(crate) use profiles_window::ProfilesWindow;
mod profiles_window_command_handler;
use profiles_window_command_handler::ProfilesWindowCommandHandler;
mod application_menu;
use application_menu::ApplicationMenu;
