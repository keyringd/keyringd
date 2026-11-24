use keyringd_core::model::ProfileId;
use keyringd_core::service::DeviceKeyProtectionChange;

#[derive(Clone)]
pub struct DeviceKeyProtectionProps {
    pub profile_id: ProfileId,
    pub profile_name: String,
    pub change: DeviceKeyProtectionChange,
}
