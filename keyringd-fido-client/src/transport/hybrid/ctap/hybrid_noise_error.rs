/// An error caused by handling a Noise handshake or encrypted message. Communication errors are handled by the channel side.
#[derive(Debug, thiserror::Error)]
pub enum HybridNoiseError {
    #[error("hybrid Noise protocol failed")]
    Noise(#[from] snow::Error),
    #[error("hybrid Noise handshake did not finish")]
    HandshakeIncomplete,
    #[error("hybrid frame exceeds the 1 MiB limit")]
    FrameTooLarge,
    #[error("hybrid frame is malformed")]
    InvalidFrame,
}
