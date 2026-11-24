/// An error indicating that the UI interaction itself failed. Holds the original cause that the handler returned.
#[derive(Debug, thiserror::Error)]
#[error("FIDO client interaction failed")]
pub struct FidoInteractionHandlerError {
    #[source]
    source: Box<dyn std::error::Error + Send + Sync>,
}

impl FidoInteractionHandlerError {
    pub fn new(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}
