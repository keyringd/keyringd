use crate::webauthn::CoseAlgorithmIdentifier;
use crate::webauthn::PackedAttestationStatement;

pub(crate) fn statement() -> PackedAttestationStatement {
    PackedAttestationStatement {
        algorithm: CoseAlgorithmIdentifier::ES256,
        signature: vec![1, 2],
        certificate_chain: None,
    }
}
