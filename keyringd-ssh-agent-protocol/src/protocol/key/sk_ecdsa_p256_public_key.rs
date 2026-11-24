use super::{EcdsaP256PublicKey, SshEllipticCurve};
use crate::SshAgentCodecError;

/// The curve, Q, and application fields of an SSH SK-ECDSA public key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkEcdsaP256PublicKey {
    q: Vec<u8>,
    application: Vec<u8>,
}

impl SkEcdsaP256PublicKey {
    pub fn new(q: &[u8], application: impl Into<Vec<u8>>) -> Result<Self, SshAgentCodecError> {
        let key = EcdsaP256PublicKey::new(q)?;
        Ok(Self {
            q: key.q().to_vec(),
            application: application.into(),
        })
    }

    pub fn curve(&self) -> SshEllipticCurve {
        SshEllipticCurve::NistP256
    }

    pub fn q(&self) -> &[u8] {
        &self.q
    }

    pub fn application(&self) -> &[u8] {
        &self.application
    }
}
