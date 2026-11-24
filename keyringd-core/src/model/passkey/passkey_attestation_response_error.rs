#[derive(Debug, thiserror::Error)]
pub enum PasskeyAttestationResponseError {
    #[error("invalid Passkey attestation encoding")]
    Protocol(#[from] keyringd_fido::ProtocolError),
    #[error("Passkey attestation has an unexpected RP, origin, ceremony or credential")]
    Context,
    #[error("Passkey attestation public key does not match the assertion")]
    Signature,
    #[error("invalid Passkey attestation storage format")]
    StorageFormat,
    #[error(transparent)]
    Cbor(#[from] keyringd_fido::CodecError),
}
