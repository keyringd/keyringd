use keyringd_fido::TransportError;
use keyringd_fido::transport::usb::CtapHidCommand;

/// An error caused by CTAPHID communication. It distinguishes between HID input/output, packet format, initialization response, and communication status returned by the authenticator.
#[derive(Debug, thiserror::Error)]
pub enum CtapHidTransportError<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    #[error("HID I/O failed")]
    Io(#[source] E),
    #[error(transparent)]
    Framing(#[from] TransportError),
    #[error("CTAPHID INIT response had {0} bytes; expected at least 17")]
    InvalidInitResponseLength(usize),
    #[error("CTAPHID INIT response nonce did not match the request")]
    InitNonceMismatch,
    #[error("authenticator allocated an invalid CTAPHID channel {0:08x}")]
    InvalidAllocatedChannel(u32),
    #[error("authenticator does not advertise CTAPHID CBOR support")]
    CborNotSupported,
    #[error("CTAPHID returned authenticator error 0x{0:02x}")]
    Authenticator(u8),
    #[error("CTAPHID {command:?} response had {actual} payload bytes; expected exactly {expected}")]
    InvalidCommandPayloadLength {
        command: CtapHidCommand,
        expected: usize,
        actual: usize,
    },
    #[error("received CTAPHID {actual:?}; expected {expected:?}")]
    UnexpectedCommand {
        expected: CtapHidCommand,
        actual: CtapHidCommand,
    },
}
