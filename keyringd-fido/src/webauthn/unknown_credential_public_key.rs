use super::CoseAlgorithmIdentifier;
use crate::EncodedCborValue;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownCredentialPublicKey {
    key_type: i64,
    algorithm: CoseAlgorithmIdentifier,
    encoded: EncodedCborValue,
}

impl UnknownCredentialPublicKey {
    pub(crate) fn new(
        key_type: i64,
        algorithm: CoseAlgorithmIdentifier,
        encoded: EncodedCborValue,
    ) -> Self {
        Self {
            key_type,
            algorithm,
            encoded,
        }
    }

    pub const fn key_type(&self) -> i64 {
        self.key_type
    }

    pub const fn algorithm(&self) -> CoseAlgorithmIdentifier {
        self.algorithm
    }

    pub fn encoded(&self) -> &EncodedCborValue {
        &self.encoded
    }
}
