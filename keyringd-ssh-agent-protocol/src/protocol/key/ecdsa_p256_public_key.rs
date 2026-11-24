use super::SshEllipticCurve;
use crate::SshAgentCodecError;

/// The Q field of an SSH nistp256 public key, retaining its SEC1 encoding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcdsaP256PublicKey {
    q: Vec<u8>,
}

impl EcdsaP256PublicKey {
    pub fn new(q: &[u8]) -> Result<Self, SshAgentCodecError> {
        p256::PublicKey::from_sec1_bytes(q).map_err(|_| SshAgentCodecError::InvalidValue {
            field: "public key",
            reason: "not a valid ECDSA P-256 point",
        })?;
        Ok(Self { q: q.to_vec() })
    }

    pub fn curve(&self) -> SshEllipticCurve {
        SshEllipticCurve::NistP256
    }

    pub fn q(&self) -> &[u8] {
        &self.q
    }
}
