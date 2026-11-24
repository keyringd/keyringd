use serde::{Deserialize, Serialize};

/// Opaque credential identifier persisted by keyringd.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CredentialId(#[serde(with = "crate::util::base64url_serde")] Vec<u8>);

impl CredentialId {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
