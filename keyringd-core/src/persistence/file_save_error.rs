#[derive(Debug, thiserror::Error)]
pub enum FileSaveError {
    #[error("file save failed before replacement: {0}")]
    BeforeReplace(#[from] std::io::Error),
    #[error("file was replaced but its durability could not be confirmed: {0}")]
    AfterReplace(#[source] std::io::Error),
}
