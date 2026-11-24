#[allow(clippy::module_inception)]
mod replace_device_key_window;
mod replace_device_key_window_command_handler;

pub(crate) use keyringd_ui_core::replace_device_key_window::ReplaceDeviceKeyProps;
pub(crate) use replace_device_key_window::ReplaceDeviceKeyWindow;
use replace_device_key_window_command_handler::ReplaceDeviceKeyWindowCommandHandler;
