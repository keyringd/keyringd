use crate::model::device_key::DeviceKeyAlgorithm;
use crate::model::device_key::DevicePrivateKeyHandle;
use crate::service::DeviceKeyProtectionChange;

#[derive(Clone, Copy, Debug)]
pub struct DeviceKeyProtectionChangeRequest<'a> {
    pub profile_name: &'a str,
    pub handle: &'a DevicePrivateKeyHandle,
    pub change: DeviceKeyProtectionChange,
    pub algorithm: DeviceKeyAlgorithm,
}
