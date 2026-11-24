#[derive(Debug, thiserror::Error)]
pub enum AuthenticatorCapabilityError {
    #[error("authenticator advertises no supported FIDO2 version")]
    UnsupportedVersion,
    #[error("authenticator does not support discoverable credentials")]
    DiscoverableCredentialsUnsupported,
    #[error("authenticator does not support any requested public-key algorithm")]
    AlgorithmUnsupported,
    #[error("authenticator does not support required extension {0}")]
    ExtensionUnsupported(String),
    #[error("CTAP request needs {required} bytes but authenticator accepts at most {maximum}")]
    MessageTooLarge { required: usize, maximum: usize },
    #[error(
        "credential list contains {actual} entries but authenticator accepts at most {maximum}"
    )]
    CredentialListTooLong { actual: usize, maximum: usize },
    #[error("no credential in the allow list can belong to this authenticator")]
    NoMatchingCredential,
    #[error("CTAP request is invalid: {0}")]
    InvalidRequest(#[source] keyringd_fido::ProtocolError),
    #[error("user verification is required but unavailable")]
    UserVerificationUnavailable,
    #[error("authenticator requires its PIN to be changed before use")]
    PinChangeRequired,
    #[error("authenticator and client share no PIN/UV protocol")]
    PinUvAuthProtocolUnsupported,
}
