/// Indicates that the PIN does not meet the input conditions such as the number of characters and the number of bytes. Differentiate from the PIN rejection by the authenticator.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PinValidationError {
    #[error("PIN must contain at least four Unicode characters")]
    TooShort,
    #[error("PIN must contain at least {minimum} Unicode characters for this authenticator")]
    AuthenticatorMinimum { minimum: u64 },
    #[error("PIN UTF-8 representation must not exceed 63 bytes")]
    TooLong,
    #[error("PIN must not end with a zero byte")]
    TrailingZero,
}
