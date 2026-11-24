use super::DeviceKeyProtectionResult;
use keyringd_core::service::DeviceKeyProtectionChange;

#[derive(Debug, Eq, PartialEq)]
pub enum DeviceKeyProtectionCommand {
    Close,
    Change(DeviceKeyProtectionChange),
    Notify(DeviceKeyProtectionResult),
    ShowRetry(String),
}
