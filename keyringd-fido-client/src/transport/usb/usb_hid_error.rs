/// Failed to enumerate, connect, and input/output USB devices. Retain backend specific causes.
#[derive(Debug, thiserror::Error)]
#[error("USB HID failed: {source}")]
pub struct UsbHidError {
    #[source]
    source: Box<dyn std::error::Error + Send + Sync>,
    disconnected: bool,
}
impl UsbHidError {
    pub fn new(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(error),
            disconnected: false,
        }
    }
    /// The backend confirmed that the opened connection was lost.
    pub fn disconnected(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(error),
            disconnected: true,
        }
    }

    pub fn is_disconnected(&self) -> bool {
        self.disconnected
    }
}
