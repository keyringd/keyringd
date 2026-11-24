#[derive(Debug, thiserror::Error)]
#[error("software device key operation failed")]
pub struct SoftwareDeviceKeyError;
