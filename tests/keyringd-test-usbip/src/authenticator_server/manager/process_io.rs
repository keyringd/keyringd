use super::AuthenticatorServerProcessError;
pub(crate) fn io(
    operation: &'static str,
    source: std::io::Error,
) -> AuthenticatorServerProcessError {
    AuthenticatorServerProcessError::Io { operation, source }
}
