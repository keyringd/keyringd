#[derive(Debug, thiserror::Error)]
pub(crate) enum SecureEnclaveDeviceKeyError {
    #[error("Secure Enclave key has an unrecognized access-control policy")]
    UnknownAccessControl,
    #[error("unsupported device-key algorithm")]
    UnsupportedAlgorithm,
    #[error("Secure Enclave key was not found")]
    KeyNotFound,
    #[error("Secure Enclave key has no public key")]
    MissingPublicKey,
    #[error("Secure Enclave public key could not be exported")]
    MissingPublicRepresentation,
    #[error("invalid Secure Enclave public key: {0}")]
    InvalidPublicKey(#[source] keyringd_ssh_agent_protocol::SshAgentCodecError),
    #[error("invalid Secure Enclave ECDSA signature: {0}")]
    InvalidSignature(#[source] p256::ecdsa::Error),
    #[error("user cancelled local authentication")]
    UserCancelled,
    #[error("LocalAuthentication context is unavailable")]
    AuthenticationContextUnavailable,
    #[error("Security.framework operation failed: {0}")]
    SecurityFramework(#[source] super::security_framework_error::SecurityFrameworkError),
}
