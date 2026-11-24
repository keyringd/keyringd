use super::CredentialPublicKey;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestedCredentialData {
    aaguid: [u8; 16],
    credential_id: Vec<u8>,
    credential_public_key: CredentialPublicKey,
}

impl AttestedCredentialData {
    pub fn new(
        aaguid: [u8; 16],
        credential_id: Vec<u8>,
        credential_public_key: CredentialPublicKey,
    ) -> Self {
        Self {
            aaguid,
            credential_id,
            credential_public_key,
        }
    }
    pub const fn aaguid(&self) -> &[u8; 16] {
        &self.aaguid
    }
    pub fn credential_id(&self) -> &[u8] {
        &self.credential_id
    }
    pub fn credential_public_key(&self) -> &CredentialPublicKey {
        &self.credential_public_key
    }
}
