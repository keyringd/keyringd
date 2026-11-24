use super::SshCertificateFields;
use crate::protocol::codec::SshWriter;
use crate::{EcdsaP256PublicKey, SshAgentCodecError, SshEllipticCurve};

/// The fields of an OpenSSH ECDSA P-256 certificate preceding its CA signature.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcdsaP256CertificateBody {
    pub nonce: Vec<u8>,
    pub q: Vec<u8>,
    pub fields: SshCertificateFields,
}

impl EcdsaP256CertificateBody {
    pub fn curve(&self) -> SshEllipticCurve {
        SshEllipticCurve::NistP256
    }

    pub(super) fn validate(&self) -> Result<(), SshAgentCodecError> {
        EcdsaP256PublicKey::new(&self.q)?;
        self.fields.validate()
    }

    pub(super) fn encode(&self, writer: &mut SshWriter) {
        writer.write_string(b"ecdsa-sha2-nistp256-cert-v01@openssh.com");
        writer.write_string(&self.nonce);
        writer.write_string(self.curve().as_bytes());
        writer.write_string(&self.q);
        self.fields.encode(writer);
    }
}
