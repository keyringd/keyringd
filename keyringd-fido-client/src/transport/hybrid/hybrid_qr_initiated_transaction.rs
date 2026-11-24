use std::time::{SystemTime, UNIX_EPOCH};

use keyringd_fido::transport::hybrid::HybridQrPayload;
use p256::{SecretKey, elliptic_curve::Generate, elliptic_curve::sec1::ToSec1Point};
use rand_core::{OsRng, RngCore};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use super::crypto as hybrid_crypto;
use super::crypto::HybridKeyPurpose;
use crate::{HybridCryptoError, HybridOperationHint, HybridSecret64};

const KNOWN_TUNNEL_DOMAIN_COUNT: u64 = 2;

/// Holds the private key, the QR secret value, and the display URI used for the Hybrid connection of the one QR origin.
/// Used to match advertisements and derive keys for connection, and erase the confidential information held when discarded.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct HybridQrInitiatedTransaction {
    #[zeroize(skip)] // SecretKey implements ZeroizeOnDrop itself.
    pub(super) private_key: SecretKey,
    pub(crate) qr_secret: [u8; 16],
    pub(super) qr_uri: String,
}

impl HybridQrInitiatedTransaction {
    pub fn new(hint: HybridOperationHint) -> Result<Self, HybridCryptoError> {
        let private_key = SecretKey::generate();
        let public_key = private_key.public_key().to_sec1_point(true);
        let public_key: [u8; 33] = public_key
            .as_bytes()
            .try_into()
            .map_err(|_| HybridCryptoError::InvalidQrKey)?;
        let mut qr_secret = [0u8; 16];
        OsRng.fill_bytes(&mut qr_secret);
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| HybridCryptoError::InvalidSystemTime)?
            .as_secs();
        let qr_uri = HybridQrPayload::new(public_key, qr_secret, KNOWN_TUNNEL_DOMAIN_COUNT, hint)
            .with_current_time(current_time)
            .to_uri()
            .map_err(|_| HybridCryptoError::QrEncoding)?;
        Ok(Self {
            private_key,
            qr_secret,
            qr_uri,
        })
    }

    pub fn qr_uri(&self) -> &str {
        &self.qr_uri
    }

    pub fn eid_key(&self) -> Result<HybridSecret64, HybridCryptoError> {
        hybrid_crypto::derive(&self.qr_secret, None, HybridKeyPurpose::Eid)
    }

    pub(crate) fn tunnel_id(&self) -> Result<[u8; 16], HybridCryptoError> {
        let derived = hybrid_crypto::derive(&self.qr_secret, None, HybridKeyPurpose::TunnelId)?;
        Ok(derived.as_bytes()[..16]
            .try_into()
            .expect("sixteen-byte slice"))
    }

    pub(crate) fn psk(
        &self,
        advertisement_plaintext: &[u8; 16],
    ) -> Result<Zeroizing<[u8; 32]>, HybridCryptoError> {
        let derived = hybrid_crypto::derive(
            &self.qr_secret,
            Some(advertisement_plaintext),
            HybridKeyPurpose::Psk,
        )?;
        Ok(Zeroizing::new(
            derived.as_bytes()[..32].try_into().expect("32-byte slice"),
        ))
    }

    pub(crate) fn private_key_bytes(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(self.private_key.to_bytes().into())
    }

    #[cfg(test)]
    pub(crate) fn noise_public_key_sec1(&self) -> [u8; 65] {
        self.private_key
            .public_key()
            .to_sec1_point(false)
            .as_bytes()
            .try_into()
            .expect("uncompressed P-256 public key is 65 bytes")
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::support::transport_hybrid_hybrid_qr_initiated_transaction::{
        digit_encode, validate_qr_uri,
    };

    use super::*;

    keyringd_testkit::scenario! {
        group "validate_qr_uri" {
            case "Accepts an unknown operation hint for forward compatibility" {
                why "The typed QR parser must retain a future operation-hint value so adding a hint does not make otherwise usable QR payloads undecodable."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"
                given "a valid hybrid QR payload whose operation hint contains an unknown two-byte value" {
                    let private_key = SecretKey::generate();
                    let public_key = private_key.public_key().to_sec1_point(true);
                    let mut encoded = HybridQrPayload::new(
                        public_key.as_bytes().try_into().unwrap(),
                        [7; 16],
                        2,
                        HybridOperationHint::GetAssertion,
                    )
                    .to_cbor_bytes()
                    .unwrap();
                    let hint = encoded.len() - 2;
                    encoded[hint..].copy_from_slice(b"zz");
                    let uri = format!("FIDO:/{}", digit_encode(&encoded));
                }
                when "the QR URI is validated" {
                    let result = validate_qr_uri(&uri);
                }
                then "the unknown hint is accepted instead of rejecting the transaction" {
                    result.unwrap();
                }
            }
        }
    }
}
