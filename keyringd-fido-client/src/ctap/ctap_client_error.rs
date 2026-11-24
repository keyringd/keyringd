use keyringd_fido::CtapStatusCode;
use keyringd_fido::ProtocolError;

/// Distinguish the communication failure of the CTAP command, the status of the authenticator, and the failure of encoding/response verification.
#[derive(Debug, thiserror::Error)]
pub enum CtapClientError {
    #[error("CTAP transport failed: {0}")]
    Transport(#[source] crate::CtapTransportError),
    #[error("authenticator returned {0}")]
    Status(CtapStatusCode),
    #[error("CTAP codec failed: {0}")]
    Codec(#[from] ProtocolError),
    #[error("authenticator returned numberOfCredentials={0}, which cannot be represented")]
    CredentialCountTooLarge(u64),
    #[error("selection-only authenticatorGetAssertion unexpectedly succeeded")]
    UnexpectedSelectionAssertion,
    #[error("authenticatorGetNextAssertion unexpectedly returned numberOfCredentials")]
    NumberOfCredentialsOnNextAssertion,
    #[error("an allow-listed authenticatorGetAssertion unexpectedly returned numberOfCredentials")]
    NumberOfCredentialsWithAllowList,
    #[error("an allow-listed authenticatorGetAssertion unexpectedly returned userSelected")]
    UserSelectedWithAllowList,
    #[error("authenticatorGetNextAssertion unexpectedly returned userSelected")]
    UserSelectedOnNextAssertion,
}
