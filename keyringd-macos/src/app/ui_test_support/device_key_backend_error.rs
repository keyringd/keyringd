use crate::system::device_key::SecureEnclaveDeviceKeyError;
use keyringd_core::test_support::SoftwareDeviceKeyError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum DeviceKeyBackendError {
    #[error(transparent)]
    Native(#[from] SecureEnclaveDeviceKeyError),
    #[error(transparent)]
    Test(#[from] SoftwareDeviceKeyError),
}
