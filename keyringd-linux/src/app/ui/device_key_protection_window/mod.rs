#[allow(clippy::module_inception)]
mod device_key_protection_window;
pub(crate) use device_key_protection_window::DeviceKeyProtectionWindow;
mod device_key_protection_command_handler;
use device_key_protection_command_handler::DeviceKeyProtectionCommandHandler;
