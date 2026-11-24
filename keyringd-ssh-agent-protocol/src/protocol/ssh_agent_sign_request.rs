use super::SshAgentSignFlags;
use crate::SshPublicKeyBlob;
use crate::protocol::codec::{SshAgentCodecError, SshReader};

#[derive(Debug)]
pub struct SshAgentSignRequest {
    key_blob: SshPublicKeyBlob,
    data: Vec<u8>,
    flags: SshAgentSignFlags,
}

impl SshAgentSignRequest {
    pub fn new(key_blob: SshPublicKeyBlob, data: Vec<u8>, flags: SshAgentSignFlags) -> Self {
        Self {
            key_blob,
            data,
            flags,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SshAgentCodecError> {
        let mut reader = SshReader::new(bytes);
        let key_blob = read_string(&mut reader, "key blob")?;
        let data = read_string(&mut reader, "data")?;
        let flags = reader
            .read_u32()
            .map_err(|_| SshAgentCodecError::Truncated { field: "flags" })?;

        let request = Self {
            key_blob: SshPublicKeyBlob::from_bytes(key_blob)?,
            data: data.to_vec(),
            flags: SshAgentSignFlags::from_bits(flags),
        };
        let trailing_bytes = reader.remaining().len();
        reader
            .finish()
            .map_err(|_| SshAgentCodecError::TrailingData { trailing_bytes })?;
        Ok(request)
    }

    pub fn key_blob(&self) -> &SshPublicKeyBlob {
        &self.key_blob
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn flags(&self) -> SshAgentSignFlags {
        self.flags
    }
}

fn read_string<'a>(
    reader: &mut SshReader<'a>,
    field: &'static str,
) -> Result<&'a [u8], SshAgentCodecError> {
    reader
        .read_string()
        .map_err(|_| SshAgentCodecError::Truncated { field })
}
