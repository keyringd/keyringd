use crate::model::device_key::DeviceKeyAlgorithm;
use crate::model::device_key::LocalAuthentication;
use crate::service::DeviceKeyGenerationReason;

#[derive(Clone, Copy, Debug)]
pub struct DeviceKeyGenerationRequest<'a> {
    pub profile_name: &'a str,
    pub reason: DeviceKeyGenerationReason,
    pub algorithm: DeviceKeyAlgorithm,
    pub local_authentication: LocalAuthentication,
}
