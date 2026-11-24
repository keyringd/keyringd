use crate::model::device_key::LocalAuthentication;

/// Returned only after signing and required local authentication succeed.
pub struct DeviceKeySignature {
    pub signature: Vec<u8>,
    pub local_authentication: LocalAuthentication,
}
