#[derive(Debug, thiserror::Error)]
#[error("mock device key store error")]
pub(crate) struct MockDeviceKeyServiceError;
