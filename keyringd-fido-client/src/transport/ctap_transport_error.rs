/// CTAP Transport failed to communicate. Retain due to USB/HYBRID-specific errors.
#[derive(Debug, thiserror::Error)]
#[error("CTAP transport failed: {source}")]
pub struct CtapTransportError {
    #[source]
    source: Box<dyn std::error::Error + Send + Sync>,
}
impl CtapTransportError {
    pub fn new(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(error),
        }
    }
}
