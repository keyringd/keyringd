use keyringd_fido::ctap::PinUvKeyAgreementKey;
use p256::{
    PublicKey, ecdh::EphemeralSecret, elliptic_curve::Generate, elliptic_curve::sec1::ToSec1Point,
};
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::crypto as pin_uv_auth_crypto;
use super::{PinUvAuthCryptoError, PinUvAuthProtocol, PinUvAuthToken};

/// Shared key agreed on a single PIN/UV key exchange. It is used for encryption, authentication, and token decryption, and erased when discarded.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct PinUvSharedSecret {
    #[zeroize(skip)]
    protocol: PinUvAuthProtocol,
    value: Vec<u8>,
}

impl PinUvSharedSecret {
    pub(crate) fn establish(
        protocol: PinUvAuthProtocol,
        peer_key: &PinUvKeyAgreementKey,
    ) -> Result<(PinUvKeyAgreementKey, Self), PinUvAuthCryptoError> {
        let peer = PublicKey::from_sec1_bytes(&peer_key.to_sec1_public_key())
            .map_err(|error| PinUvAuthCryptoError::InvalidPeerPublicKey(error.into()))?;
        let private_key = EphemeralSecret::generate();
        let public_key = private_key.public_key().to_sec1_point(false);
        let own_sec1: [u8; 65] =
            public_key
                .as_bytes()
                .try_into()
                .map_err(|error: std::array::TryFromSliceError| {
                    PinUvAuthCryptoError::InvalidPeerPublicKey(error.into())
                })?;
        let platform_key = PinUvKeyAgreementKey::from_sec1_public_key(&own_sec1)
            .map_err(|error| PinUvAuthCryptoError::InvalidPeerPublicKey(error.into()))?;
        let raw = private_key.diffie_hellman(&peer);
        let value = pin_uv_auth_crypto::derive_secret(protocol, raw.raw_secret_bytes())?;
        Ok((platform_key, Self::new(protocol, value)))
    }

    pub(crate) fn new(protocol: PinUvAuthProtocol, value: Vec<u8>) -> Self {
        Self { protocol, value }
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, PinUvAuthCryptoError> {
        pin_uv_auth_crypto::encrypt(self.protocol, &self.value, plaintext)
    }

    pub fn authenticate(&self, message: &[u8]) -> Result<Vec<u8>, PinUvAuthCryptoError> {
        pin_uv_auth_crypto::authenticate(self.protocol, &self.value, message)
    }

    pub fn decrypt_token(&self, encrypted: &[u8]) -> Result<PinUvAuthToken, PinUvAuthCryptoError> {
        PinUvAuthToken::new(
            self.protocol,
            pin_uv_auth_crypto::decrypt(self.protocol, &self.value, encrypted)?,
        )
    }

    #[cfg(test)]
    pub(crate) fn expose_bytes(&self) -> &[u8] {
        &self.value
    }
}
