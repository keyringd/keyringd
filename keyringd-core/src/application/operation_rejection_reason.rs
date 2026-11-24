/// Why an application operation cannot be accepted, due to its arguments or current state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum OperationRejectionReason {
    #[error("Passkeys require an SK-ECDSA or WebAuthn signature algorithm")]
    UnsupportedPasskeySignatureAlgorithm,
    #[error("unsupported Device Key signature algorithm")]
    UnsupportedDeviceKeySignatureAlgorithm,
    #[error("the profile is disabled")]
    ProfileDisabled,
    #[error("the selected SSH identity is no longer available")]
    SshIdentityUnavailable,
    #[error("the SSH signing request is invalid or unauthorized")]
    InvalidSshSigningRequest,
    #[error(
        "profile name must contain 1 to 80 characters without surrounding whitespace, line breaks, or control characters"
    )]
    InvalidProfileName,
    #[error("profile order must contain every profile exactly once")]
    ProfileOrderLength,
    #[error("profile order contains an unknown profile")]
    UnknownProfileInOrder,
    #[error("profile order contains a duplicate profile")]
    DuplicateProfileInOrder,
    #[error("credential selection is required to recover this Passkey")]
    PasskeySelectionRequired,
    #[error("selected credential was not offered by the authenticator")]
    PasskeySelectionNotOffered,
}
