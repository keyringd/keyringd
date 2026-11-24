use crate::system::device_key::CngError;
use keyringd_core::test_support::SoftwareDeviceKeyError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum DeviceKeyBackendError {
    #[error(transparent)]
    Fixture(#[from] anyhow::Error),
    #[error(transparent)]
    Native(#[from] CngError),
    #[error(transparent)]
    Test(#[from] SoftwareDeviceKeyError),
}
