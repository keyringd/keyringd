use crate::protocol::codec::SshAgentCodecError;

#[derive(Debug, thiserror::Error)]
pub enum SshSigningPayloadError {
    #[error("SSH signing payload is truncated at {field}")]
    Truncated { field: &'static str },
    #[error("SSH signing payload contains {trailing_bytes} trailing bytes")]
    TrailingData { trailing_bytes: usize },
    #[error("data is neither a valid SSH userauth request nor an SSHSIG payload")]
    Unsupported,
    #[error("malformed SSHSIG payload: {0}")]
    MalformedSshsig(&'static str),
    #[error("SSH userauth session_id is empty")]
    EmptySessionId,
    #[error("expected SSH_MSG_USERAUTH_REQUEST (50), got {actual}")]
    NotUserAuthRequest { actual: u8 },
    #[error("expected service ssh-connection")]
    InvalidService,
    #[error("expected a public-key authentication method")]
    NotPublicKeyMethod,
    #[error("publickey boolean is {actual}, expected 1")]
    PublicKeyBooleanFalse { actual: u8 },
    #[error("{field} is not a valid SSH key blob")]
    InvalidKeyBlob {
        field: &'static str,
        #[source]
        source: SshAgentCodecError,
    },
    #[error("userauth algorithm does not match the embedded key blob")]
    KeyAlgorithmMismatch,
}
