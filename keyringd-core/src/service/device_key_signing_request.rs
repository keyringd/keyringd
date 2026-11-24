use crate::model::device_key::{DevicePrivateKeyHandle, LocalAuthentication};

pub struct DeviceKeySigningRequest<'a> {
    pub handle: &'a DevicePrivateKeyHandle,
    pub profile_name: &'a str,
    /// Called with the authentication requirement of the actual signing key.
    pub digest: &'a (dyn Fn(LocalAuthentication) -> [u8; 32] + Send + Sync),
}
