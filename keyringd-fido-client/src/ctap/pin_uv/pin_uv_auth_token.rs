use zeroize::{Zeroize, ZeroizeOnDrop};

use super::crypto as pin_uv_auth_crypto;
use crate::{PinUvAuthCryptoError, PinUvAuthProtocol};

/// PIN/UV certified token obtained from the authenticator. Authenticate the operation request and delete it when discarded.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct PinUvAuthToken {
    #[zeroize(skip)]
    protocol: PinUvAuthProtocol,
    value: Vec<u8>,
}

impl PinUvAuthToken {
    pub(crate) fn new(
        protocol: PinUvAuthProtocol,
        value: Vec<u8>,
    ) -> Result<Self, PinUvAuthCryptoError> {
        let valid = match protocol {
            PinUvAuthProtocol::One => matches!(value.len(), 16 | 32),
            PinUvAuthProtocol::Two => value.len() == 32,
        };
        if !valid {
            return Err(PinUvAuthCryptoError::InvalidLength {
                protocol: protocol.version(),
            });
        }
        Ok(Self { protocol, value })
    }

    pub fn authenticate(&self, message: &[u8]) -> Result<Vec<u8>, PinUvAuthCryptoError> {
        pin_uv_auth_crypto::authenticate(self.protocol, &self.value, message)
    }

    pub(crate) fn protocol(&self) -> PinUvAuthProtocol {
        self.protocol
    }
}
