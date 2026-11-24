/// Indicates that the format of the authenticator response and the data required to assemble the results are incorrect.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ResponseValidationError {
    #[error(
        "assertion omitted its credential and the request does not identify exactly one credential"
    )]
    MissingCredential,
    #[error("assertion client data does not match the prepared request")]
    ClientDataMismatch,
    #[error("assertion RP ID hash does not match the requested RP")]
    RpIdMismatch,
    #[error("assertion returned an unexpected or empty credential")]
    UnexpectedCredential,
    #[error("assertion did not prove required user presence")]
    UserPresenceMissing,
    #[error("assertion did not prove required user verification")]
    UserVerificationMissing,
    #[error("invalid authenticator assertion data or empty signature")]
    InvalidAssertion,
    #[error("assertion credential count does not match the responses")]
    CredentialCountMismatch,
    #[error("makeCredential response omitted attested credential data")]
    MissingAttestedCredentialData,
    #[error("authenticator response contains an invalid WebAuthn extension identifier: {0}")]
    InvalidExtensionIdentifier(String),
    #[error("authenticator response contains an invalid attestation format identifier: {0}")]
    InvalidAttestationFormatIdentifier(String),
    #[error("credential public key contains forbidden optional COSE Key parameter {0}")]
    ForbiddenCredentialPublicKeyParameter(i64),
    #[error("discoverable getAssertion response omitted the user entity")]
    MissingDiscoverableUserEntity,
    #[error("getAssertion returned the same credential more than once")]
    DuplicateCredential,
    #[error("authenticator returned no assertions")]
    NoAssertions,
}
