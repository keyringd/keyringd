use crate::SshPublicKeyBlob;
use crate::protocol::codec::SshWriter;

/// A public key or certificate offered by the agent, with its display comment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SshAgentIdentity {
    key_blob: SshPublicKeyBlob,
    comment: String,
}

impl SshAgentIdentity {
    pub fn new(key_blob: SshPublicKeyBlob, comment: impl Into<String>) -> Self {
        Self {
            key_blob,
            comment: comment.into(),
        }
    }

    pub fn key_blob(&self) -> &SshPublicKeyBlob {
        &self.key_blob
    }

    pub fn comment(&self) -> &str {
        &self.comment
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = SshWriter::new();
        writer.write_string(self.key_blob.as_bytes());
        writer.write_string(self.comment.as_bytes());
        writer.into_bytes()
    }
}
