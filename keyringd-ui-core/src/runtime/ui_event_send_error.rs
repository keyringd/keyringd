/// The UI event receiver has been disposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiEventSendError;
impl std::fmt::Display for UiEventSendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UI event receiver is closed")
    }
}
impl std::error::Error for UiEventSendError {}
