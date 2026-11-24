use keyringd_fido::{ProtocolError, TransportError};

use crate::HybridNoiseError;

/// Errors caused by CTAP communication on Hybrid. Distinguish between cryptographic communications, connection status, data channels, and response format problems.
#[derive(Debug, thiserror::Error)]
pub enum HybridCtapTransportError<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    #[error(transparent)]
    Noise(#[from] HybridNoiseError),
    #[error("hybrid data channel failed")]
    Channel(#[source] E),
    #[error("hybrid data channel closed")]
    ChannelClosed,
    #[error("hybrid initial message is malformed")]
    InvalidInitialMessage(#[source] ProtocolError),
    #[error("hybrid peer does not advertise CTAP support")]
    CtapUnsupported,
    #[error("hybrid initial GetInfo response is invalid")]
    InvalidGetInfo(#[source] ProtocolError),
    #[error("hybrid CTAP response is malformed")]
    InvalidCtapResponse(#[source] TransportError),
    #[error("hybrid tunnel frame is malformed")]
    InvalidFrame,
    #[error("authenticator sent a shutdown message, which is not permitted")]
    UnexpectedShutdown,
    #[error("hybrid tunnel sent unsupported message type {0}")]
    UnsupportedMessageType(u8),
}
