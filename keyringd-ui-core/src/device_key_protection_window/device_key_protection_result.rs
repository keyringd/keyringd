use keyringd_core::model::DeviceKeyRegistrationNotice;
use keyringd_core::model::Profile;
use keyringd_core::service::DeviceKeyProtectionChange;

/// Successful protection change and the information to present to the user.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceKeyProtectionResult {
    pub notice: String,
    pub registration_notice: Option<DeviceKeyRegistrationNotice>,
}
impl DeviceKeyProtectionResult {
    pub fn between(
        change: DeviceKeyProtectionChange,
        previous: &Profile,
        current: &Profile,
    ) -> Self {
        Self {
            notice: crate::device_key_protection_window::succeeded(change).to_owned(),
            registration_notice: DeviceKeyRegistrationNotice::between(previous, current),
        }
    }
}
