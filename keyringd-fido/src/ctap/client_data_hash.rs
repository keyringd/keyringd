use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ClientDataHash([u8; Self::LENGTH]);

impl ClientDataHash {
    pub const LENGTH: usize = 32;

    pub const fn new(bytes: [u8; Self::LENGTH]) -> Self {
        Self(bytes)
    }

    pub fn hash(client_data: &[u8]) -> Self {
        Self(Sha256::digest(client_data).into())
    }

    pub const fn as_bytes(&self) -> &[u8; Self::LENGTH] {
        &self.0
    }

    pub const fn into_bytes(self) -> [u8; Self::LENGTH] {
        self.0
    }
}

impl From<[u8; ClientDataHash::LENGTH]> for ClientDataHash {
    fn from(value: [u8; ClientDataHash::LENGTH]) -> Self {
        Self::new(value)
    }
}

impl From<ClientDataHash> for [u8; ClientDataHash::LENGTH] {
    fn from(value: ClientDataHash) -> Self {
        value.into_bytes()
    }
}
