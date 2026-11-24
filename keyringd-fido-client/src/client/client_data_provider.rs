use super::{ClientData, ClientDataProviderError};

/// An application-implemented interface that generates a signed ClientData from a challenge.
/// Implementation is passed per operation. A closure can also be used as an implementation of this trait.
pub trait ClientDataProvider: Send + Sync {
    fn provide(&self, challenge: &[u8]) -> Result<ClientData, ClientDataProviderError>;
}

impl<F> ClientDataProvider for F
where
    F: Fn(&[u8]) -> Result<ClientData, ClientDataProviderError> + Send + Sync,
{
    fn provide(&self, challenge: &[u8]) -> Result<ClientData, ClientDataProviderError> {
        self(challenge)
    }
}

/// Supplies previously collected data for retries of the same ceremony.
impl ClientDataProvider for ClientData {
    fn provide(&self, _challenge: &[u8]) -> Result<ClientData, ClientDataProviderError> {
        Ok(self.clone())
    }
}
