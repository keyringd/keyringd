/// Errors caused by operating the BLE adapter and processing of receiving advertisements. Retain the original cause of the OS/Bluetooth library.
#[derive(Debug, thiserror::Error)]
pub enum HybridBleError {
    #[error("Bluetooth Low Energy operation failed")]
    Platform(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl HybridBleError {
    pub(crate) fn platform(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Platform(Box::new(error))
    }
}
