use super::PinValidationError;

use unicode_normalization::UnicodeNormalization;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A row of bytes of PIN to pass to the authenticator. Verify the format when generated and erase the byte strings that are retained at the time of discard.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Pin(Vec<u8>);

impl Pin {
    pub fn from_text(value: &str) -> Result<Self, PinValidationError> {
        Self::from_text_with_minimum(value, 4)
    }

    pub fn from_text_with_minimum(value: &str, minimum: u64) -> Result<Self, PinValidationError> {
        let normalized = value.nfc().collect::<String>();
        let characters = normalized.chars().count();
        if characters < 4 {
            return Err(PinValidationError::TooShort);
        }
        if u64::try_from(characters).unwrap_or(u64::MAX) < minimum {
            return Err(PinValidationError::AuthenticatorMinimum { minimum });
        }
        let bytes = normalized.into_bytes();
        if bytes.len() > 63 {
            return Err(PinValidationError::TooLong);
        }
        if bytes.last() == Some(&0) {
            return Err(PinValidationError::TrailingZero);
        }
        Ok(Self(bytes))
    }

    pub fn expose_bytes(&self) -> &[u8] {
        &self.0
    }
}
