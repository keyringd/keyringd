use crate::{CtapClientError, PinUvAuthCryptoError};

/// An error indicating that the communication, encryption processing, or insufficient response of the PIN/UV authentication procedure is insufficient.
#[derive(Debug, thiserror::Error)]
pub enum PinUvAuthError {
    #[error(transparent)]
    Client(#[from] CtapClientError),
    #[error(transparent)]
    Crypto(#[from] PinUvAuthCryptoError),
    #[error("authenticatorClientPIN response omitted {0}")]
    MissingResponseField(&'static str),
}
