/// Errors caused by WebSocket connection/communication to a Hybrid tunnel.
/// In addition to the cause of the communications library, it distinguishes bad redirects, HTTP responses, and unexpected messages.
#[derive(Debug, thiserror::Error)]
pub enum HybridWebSocketError {
    #[error("hybrid tunnel connection timed out after 15 seconds")]
    ConnectionTimedOut,
    #[error("hybrid tunnel WebSocket failed")]
    WebSocket(#[source] Box<tokio_tungstenite::tungstenite::Error>),
    #[error("hybrid tunnel redirected without a valid Location header")]
    InvalidRedirect(#[source] Option<Box<dyn std::error::Error + Send + Sync>>),
    #[error("hybrid tunnel exceeded the redirect limit")]
    TooManyRedirects,
    #[error("hybrid tunnel rejected the connection with HTTP status {0}")]
    HttpStatus(u16),
    #[error("hybrid tunnel sent a non-binary WebSocket message")]
    UnexpectedMessage,
}

impl HybridWebSocketError {
    pub(crate) fn web_socket(error: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(Box::new(error))
    }
}
