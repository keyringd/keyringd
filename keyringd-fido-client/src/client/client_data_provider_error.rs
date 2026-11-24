/// An error that indicates that the application-side ClientData generation process has failed. Maintain the original cause.
#[derive(Debug, thiserror::Error)]
#[error("client data provider failed")]
pub struct ClientDataProviderError {
    #[source]
    source: Box<dyn std::error::Error + Send + Sync>,
}

impl ClientDataProviderError {
    pub fn new(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}
