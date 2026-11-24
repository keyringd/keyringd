#[derive(Debug, thiserror::Error)]
pub(crate) enum PasskeyError {
    #[error("Passkey attestation does not match its credential ID or public key")]
    AttestationMismatch,
    #[error("Passkeys require an SK-ECDSA or WebAuthn signature algorithm")]
    UnsupportedSignatureAlgorithm,
    #[error("passkey user handle has length {actual}; expected {expected}")]
    InvalidUserHandleLength { expected: usize, actual: usize },
    #[error("passkey is not a valid ECDSA P-256 public key")]
    InvalidPublicKey(#[source] p256::elliptic_curve::Error),
}
