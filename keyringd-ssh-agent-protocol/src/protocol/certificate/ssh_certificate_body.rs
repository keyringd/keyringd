use super::{
    EcdsaP256CertificateBody, SkEcdsaP256CertificateBody, SshCertificateFields,
    SshCertificateSigningRequest,
};
use crate::protocol::codec::SshWriter;
use crate::{SkEcdsaP256PublicKey, SshAgentCodecError, SshKeyAlgorithm, SshPublicKey};

/// Algorithm-specific certificate fields covered by the CA signature.
/// Subject fields are inline wire fields, never another public key blob.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SshCertificateBody {
    EcdsaP256(EcdsaP256CertificateBody),
    SkEcdsaP256(SkEcdsaP256CertificateBody),
}

impl SshCertificateBody {
    /// Build the certificate's inline subject fields from a standalone public key.
    ///
    /// A certificate blob cannot be passed as the subject:
    /// ```compile_fail
    /// use keyringd_ssh_agent_protocol::{SshCertificate, SshCertificateBody, SshCertificateFields};
    /// fn nested(certificate: SshCertificate, fields: SshCertificateFields) {
    ///     SshCertificateBody::from_public_key(vec![], certificate.to_key_blob(), fields);
    /// }
    /// ```
    pub fn from_public_key(
        nonce: Vec<u8>,
        key: SshPublicKey,
        fields: SshCertificateFields,
    ) -> Self {
        match key {
            SshPublicKey::EcdsaP256(key) => Self::EcdsaP256(EcdsaP256CertificateBody {
                nonce,
                q: key.q().to_vec(),
                fields,
            }),
            SshPublicKey::SkEcdsaP256(key) => Self::SkEcdsaP256(SkEcdsaP256CertificateBody {
                nonce,
                q: key.q().to_vec(),
                application: key.application().to_vec(),
                fields,
            }),
        }
    }

    pub fn prepare(self) -> Result<SshCertificateSigningRequest, SshAgentCodecError> {
        self.validate()?;
        let signing_bytes = self.encode();
        Ok(SshCertificateSigningRequest::new(self, signing_bytes))
    }

    pub fn nonce(&self) -> &[u8] {
        match self {
            Self::EcdsaP256(body) => &body.nonce,
            Self::SkEcdsaP256(body) => &body.nonce,
        }
    }

    pub fn fields(&self) -> &SshCertificateFields {
        match self {
            Self::EcdsaP256(body) => &body.fields,
            Self::SkEcdsaP256(body) => &body.fields,
        }
    }

    pub fn fields_mut(&mut self) -> &mut SshCertificateFields {
        match self {
            Self::EcdsaP256(body) => &mut body.fields,
            Self::SkEcdsaP256(body) => &mut body.fields,
        }
    }

    /// Construct a standalone public key from the inline subject fields.
    pub fn public_key(&self) -> Result<SshPublicKey, SshAgentCodecError> {
        match self {
            Self::EcdsaP256(body) => SshPublicKey::from_ecdsa_pubkey(&body.q),
            Self::SkEcdsaP256(body) => SkEcdsaP256PublicKey::new(&body.q, body.application.clone())
                .map(SshPublicKey::SkEcdsaP256),
        }
    }

    pub fn algorithm(&self) -> SshKeyAlgorithm {
        match self {
            Self::EcdsaP256(_) => SshKeyAlgorithm::EcdsaSha2NistP256Certificate,
            Self::SkEcdsaP256(_) => SshKeyAlgorithm::SkEcdsaSha2NistP256Certificate,
        }
    }

    pub(super) fn validate(&self) -> Result<(), SshAgentCodecError> {
        match self {
            Self::EcdsaP256(body) => body.validate(),
            Self::SkEcdsaP256(body) => body.validate(),
        }
    }

    pub(super) fn encode(&self) -> Vec<u8> {
        let mut writer = SshWriter::new();
        match self {
            Self::EcdsaP256(body) => body.encode(&mut writer),
            Self::SkEcdsaP256(body) => body.encode(&mut writer),
        }
        writer.into_bytes()
    }
}
