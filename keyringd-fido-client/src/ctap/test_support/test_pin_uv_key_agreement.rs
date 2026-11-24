use keyringd_fido::ctap::PinUvKeyAgreementKey;
use p256::{
    PublicKey, ecdh::EphemeralSecret, elliptic_curve::Generate, elliptic_curve::sec1::ToSec1Point,
};

use crate::ctap::pin_uv::crypto as pin_uv_auth_crypto;
use crate::ctap::pin_uv::{PinUvAuthCryptoError, PinUvAuthProtocol, PinUvSharedSecret};

/// A test key pair that returns the authenticator's public key first and agrees with the client public key of the subsequent request.
pub struct TestPinUvKeyAgreement {
    protocol: PinUvAuthProtocol,
    private_key: EphemeralSecret,
    public_key: PublicKey,
}

impl TestPinUvKeyAgreement {
    pub fn new(protocol: PinUvAuthProtocol) -> Self {
        let private_key = EphemeralSecret::generate();
        let public_key = private_key.public_key();
        Self {
            protocol,
            private_key,
            public_key,
        }
    }

    pub fn public_key(&self) -> Result<PinUvKeyAgreementKey, PinUvAuthCryptoError> {
        let encoded = self.public_key.to_sec1_point(false);
        let own_sec1: [u8; 65] =
            encoded
                .as_bytes()
                .try_into()
                .map_err(|error: std::array::TryFromSliceError| {
                    PinUvAuthCryptoError::InvalidPeerPublicKey(error.into())
                })?;
        PinUvKeyAgreementKey::from_sec1_public_key(&own_sec1)
            .map_err(|error| PinUvAuthCryptoError::InvalidPeerPublicKey(error.into()))
    }

    pub fn establish(
        &self,
        peer_key: &PinUvKeyAgreementKey,
    ) -> Result<(PinUvKeyAgreementKey, PinUvSharedSecret), PinUvAuthCryptoError> {
        let peer_sec1 = peer_key.to_sec1_public_key();
        let peer = PublicKey::from_sec1_bytes(&peer_sec1)
            .map_err(|error| PinUvAuthCryptoError::InvalidPeerPublicKey(error.into()))?;
        let raw = self.private_key.diffie_hellman(&peer);
        let secret = pin_uv_auth_crypto::derive_secret(self.protocol, raw.raw_secret_bytes())?;
        Ok((
            self.public_key()?,
            PinUvSharedSecret::new(self.protocol, secret),
        ))
    }
}
