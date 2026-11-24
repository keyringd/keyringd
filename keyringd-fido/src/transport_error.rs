#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TransportError {
    #[error("CTAP response did not contain a status byte")]
    MissingCtapStatus,
    #[error("CTAPHID packet has {actual} bytes; expected {expected}")]
    InvalidPacketSize { expected: usize, actual: usize },
    #[error(
        "CTAPHID packet size {actual} is smaller than the {minimum}-byte initialization header"
    )]
    PacketSizeTooSmall { minimum: usize, actual: usize },
    #[error("CTAPHID message declares an empty command byte")]
    InvalidCommand,
    #[error("CTAPHID message length {actual} exceeds maximum {maximum}")]
    MessageTooLarge { maximum: usize, actual: usize },
    #[error("received a continuation packet before an initialization packet")]
    UnexpectedContinuation,
    #[error("received an initialization packet while a message was incomplete")]
    UnexpectedInitialization,
    #[error("CTAPHID packet changed channel from {expected:08x} to {actual:08x}")]
    ChannelChanged { expected: u32, actual: u32 },
    #[error("CTAPHID continuation sequence was {actual}; expected {expected}")]
    InvalidSequence { expected: u8, actual: u8 },
    #[error("CTAPHID continuation sequence exceeded 127")]
    SequenceOverflow,
    #[error("CTAPHID INIT response has invalid length {actual}, expected at least 17")]
    InvalidInitResponseLength { actual: usize },
    #[error("hybrid plaintext frame is empty or malformed")]
    InvalidHybridFrame,
    #[error("hybrid plaintext frame has unsupported message type {0}")]
    UnsupportedHybridMessageType(u8),
}
