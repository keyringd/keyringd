#[derive(Debug, thiserror::Error)]
pub enum AuthenticatorServerError {
    #[error("authenticator server I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid authenticator server JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("authenticator server returned HTTP {status}: {message}")]
    Http { status: u16, message: String },
    #[error("invalid authenticator server response: {0}")]
    Protocol(String),
    #[error("control endpoint must be an http:// IP address and port")]
    InvalidEndpoint,
    #[error("authenticator ID must be an integer between 1 and 65535")]
    InvalidIdentifier,
    #[error("authenticator server request or consent wait timed out")]
    TimedOut,
}
