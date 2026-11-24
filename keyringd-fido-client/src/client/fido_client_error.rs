use keyringd_fido::CtapStatusCode;

/// An error that the caller can classify for registration/authentication failures.
/// Cancels, expires, CTAP status codes, and different causes for each layer.
#[derive(Debug, thiserror::Error)]
pub enum FidoClientError {
    #[error("FIDO operation was cancelled")]
    Cancelled,
    #[error("OS authenticator UI was cancelled")]
    OsCancelled,
    #[error("FIDO operation timed out")]
    TimedOut,
    #[error("hybrid tunnel connection timed out")]
    HybridTunnelConnectionTimedOut(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("FIDO client is busy")]
    Busy,
    #[error("no suitable authenticator was found")]
    NoAuthenticator,
    #[error("authenticator PIN must be changed before use")]
    PinChangeRequired(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("authenticator does not support required capability: {0}")]
    UnsupportedRequirement(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("FIDO interaction failed")]
    Interaction(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("FIDO transport failed")]
    Transport(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("authenticator returned CTAP status {0:?}")]
    Ctap(CtapStatusCode),
    #[error("FIDO protocol failed")]
    Protocol(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("platform FIDO backend failed")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl FidoClientError {
    pub fn pin_change_required(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::PinChangeRequired(Box::new(error))
    }
    pub fn backend(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Backend(Box::new(error))
    }
    pub(crate) fn interaction(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Interaction(Box::new(error))
    }
    pub(crate) fn transport(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Transport(Box::new(error))
    }
    pub(crate) fn protocol(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Protocol(Box::new(error))
    }
}

impl From<crate::FidoInteractionHandlerError> for FidoClientError {
    fn from(error: crate::FidoInteractionHandlerError) -> Self {
        Self::interaction(error)
    }
}

impl From<crate::CtapClientError> for FidoClientError {
    fn from(error: crate::CtapClientError) -> Self {
        match error {
            crate::CtapClientError::Transport(error) => FidoClientError::transport(error),
            crate::CtapClientError::Status(status) => FidoClientError::Ctap(status),
            other => FidoClientError::protocol(other),
        }
    }
}

impl From<crate::PinUvAuthError> for FidoClientError {
    fn from(error: crate::PinUvAuthError) -> Self {
        match error {
            crate::PinUvAuthError::Client(error) => Self::from(error),
            other => FidoClientError::protocol(other),
        }
    }
}

impl From<crate::AuthenticatorCapabilityError> for FidoClientError {
    fn from(error: crate::AuthenticatorCapabilityError) -> Self {
        match error {
            crate::AuthenticatorCapabilityError::NoMatchingCredential => {
                Self::Ctap(CtapStatusCode::NoCredentials)
            }
            other => Self::UnsupportedRequirement(other.into()),
        }
    }
}

impl From<crate::HybridConnectionError> for FidoClientError {
    fn from(error: crate::HybridConnectionError) -> Self {
        match error {
            crate::HybridConnectionError::Cancelled => Self::Cancelled,
            crate::HybridConnectionError::WebSocket(
                error @ crate::HybridWebSocketError::ConnectionTimedOut,
            ) => Self::HybridTunnelConnectionTimedOut(Box::new(error)),
            crate::HybridConnectionError::Crypto(error) => Self::protocol(error),
            error @ crate::HybridConnectionError::InvalidTunnelDomain => Self::protocol(error),
            other => Self::transport(other),
        }
    }
}

impl From<CtapStatusCode> for FidoClientError {
    fn from(status: CtapStatusCode) -> Self {
        Self::Ctap(status)
    }
}
