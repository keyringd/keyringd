/// Selects guidance for choosing an authenticator, approving, or verifying the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthenticatorPromptKind {
    AuthenticatorSelection,
    UserPresence,
    BuiltInUserVerification,
}
