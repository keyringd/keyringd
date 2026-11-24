use crate::model::device_key::{DevicePrivateKeyHandle, DevicePublicKey};

/// Key material returned by a platform key operation.
pub struct DeviceKeyMaterial {
    pub handle: DevicePrivateKeyHandle,
    pub public_key: DevicePublicKey,
}
