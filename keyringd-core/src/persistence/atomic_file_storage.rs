use super::FileSaveError;

/// Stores bytes without interpreting their document format.
#[cfg_attr(test, mockall::automock)]
pub trait AtomicFileStorage: Send + Sync {
    fn read(&self) -> std::io::Result<Option<Vec<u8>>>;
    fn replace(&self, value: &[u8]) -> Result<(), FileSaveError>;
}
