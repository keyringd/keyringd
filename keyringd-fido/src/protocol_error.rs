use crate::CodecError;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error("cannot encode credential public key as DER SubjectPublicKeyInfo")]
    PublicKeyEncoding(#[from] spki::der::Error),
    #[error("expected {expected} at {field}")]
    UnexpectedType {
        field: &'static str,
        expected: &'static str,
    },
    #[error("missing required CTAP field 0x{0:02x}")]
    MissingCtapField(u64),
    #[error("missing required field {0}")]
    MissingField(&'static str),
    #[error("{field} has length {actual}; expected {expected}")]
    InvalidLength {
        field: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("{field} contains a duplicate value")]
    DuplicateValue { field: &'static str },
    #[error("field {0} occurs more than once")]
    DuplicateField(String),
    #[error("{field} must not be empty")]
    EmptyValue { field: &'static str },
    #[error("{field} must be greater than zero")]
    MustBePositive { field: &'static str },
    #[error("{field} has an invalid value; expected {expected}")]
    InvalidValue {
        field: &'static str,
        expected: &'static str,
    },
    #[error("client data challenge is not canonical unpadded base64url")]
    InvalidChallenge(#[source] Option<base64::DecodeError>),
    #[error("client data origin is not a serialized RFC 6454 origin")]
    InvalidOrigin(#[source] Option<http::uri::InvalidUri>),
    #[error("authenticator data is too short")]
    AuthenticatorDataTooShort,
    #[error("authenticator data contains reserved flag bits 0x{0:02x}")]
    ReservedAuthenticatorDataFlags(u8),
    #[error("backup state is set without backup eligibility")]
    InvalidBackupFlags,
    #[error("attested credential data is truncated or inconsistent with flags")]
    InvalidAttestedCredentialData,
    #[error("credential ID exceeds the remaining authenticator data")]
    TruncatedCredentialId,
    #[error("credential ID length {actual} exceeds the WebAuthn maximum of 1023 bytes")]
    CredentialIdTooLong { actual: usize },
    #[error("authenticator extensions are invalid or inconsistent with flags")]
    InvalidAuthenticatorExtensions,
    #[error("unexpected trailing authenticator data")]
    TrailingAuthenticatorData,
    #[error("attestation object must be a CBOR map")]
    AttestationObjectNotAMap,
}
