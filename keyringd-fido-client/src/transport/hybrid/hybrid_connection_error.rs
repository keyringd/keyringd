/// Errors occurring when searching, connecting, and initializing encrypted communications of Hybrid. The caller does the conversion of the operation layer to an error.
#[derive(Debug, thiserror::Error)]
pub(crate) enum HybridConnectionError {
    #[error("hybrid connection was cancelled")]
    Cancelled,
    #[error(transparent)]
    Crypto(#[from] crate::HybridCryptoError),
    #[error(transparent)]
    Ble(#[from] crate::HybridBleError),
    #[error(transparent)]
    WebSocket(#[from] crate::HybridWebSocketError),
    #[error("hybrid advertisement contains an unknown tunnel domain")]
    InvalidTunnelDomain,
    #[error("hybrid channel initialization failed")]
    Channel(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl HybridConnectionError {
    pub(crate) fn channel(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Channel(Box::new(error))
    }
}
