use crate::protocol::codec::{SshAgentCodecError, SshReader};

#[derive(Debug)]
pub struct SshAgentExtensionRequest {
    name: String,
    data: Vec<u8>,
}

impl SshAgentExtensionRequest {
    pub fn new(name: String, data: Vec<u8>) -> Self {
        Self { name, data }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SshAgentCodecError> {
        let mut reader = SshReader::new(bytes);
        let name = reader
            .read_string()
            .map_err(|_| SshAgentCodecError::Truncated {
                field: "extension name",
            })?;

        let name = std::str::from_utf8(name)
            .map_err(|_| SshAgentCodecError::InvalidUtf8 {
                field: "extension name",
            })?
            .to_owned();
        Ok(Self {
            name,
            data: reader.remaining().to_vec(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}
