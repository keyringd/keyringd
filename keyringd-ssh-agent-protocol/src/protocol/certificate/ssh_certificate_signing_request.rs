use super::{SshCertificate, SshCertificateBody};
use crate::SshSignature;

/// Validated certificate fields and the exact bytes to sign.
/// The fields cannot be changed between preparation and completion.
pub struct SshCertificateSigningRequest {
    body: SshCertificateBody,
    signing_bytes: Vec<u8>,
}

impl SshCertificateSigningRequest {
    pub(super) fn new(body: SshCertificateBody, signing_bytes: Vec<u8>) -> Self {
        Self {
            body,
            signing_bytes,
        }
    }

    pub fn signing_bytes(&self) -> &[u8] {
        &self.signing_bytes
    }

    /// Attach the signature produced over `signing_bytes()`.
    /// This does not verify the signature.
    pub fn with_signature(self, signature: SshSignature) -> SshCertificate {
        SshCertificate::new(self.body, signature)
    }
}
