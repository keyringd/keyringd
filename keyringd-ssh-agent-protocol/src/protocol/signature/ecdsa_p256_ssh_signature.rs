use crate::SshSignature;
use crate::protocol::codec::{SshAgentCodecError, SshReader, SshWriter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcdsaP256SshSignature {
    r: Vec<u8>,
    s: Vec<u8>,
}

impl EcdsaP256SshSignature {
    pub fn new(r: impl AsRef<[u8]>, s: impl AsRef<[u8]>) -> Self {
        fn magnitude(bytes: &[u8]) -> Vec<u8> {
            bytes
                .iter()
                .position(|byte| *byte != 0)
                .map_or_else(Vec::new, |start| bytes[start..].to_vec())
        }
        Self {
            r: magnitude(r.as_ref()),
            s: magnitude(s.as_ref()),
        }
    }
    pub fn r(&self) -> &[u8] {
        &self.r
    }
    pub fn s(&self) -> &[u8] {
        &self.s
    }

    pub fn from_bytes(encoded: &[u8]) -> Result<Self, SshAgentCodecError> {
        fn read_component(
            reader: &mut SshReader<'_>,
            field: &'static str,
        ) -> Result<Vec<u8>, SshAgentCodecError> {
            let bytes = reader
                .read_string()
                .map_err(|_| SshAgentCodecError::Truncated { field })?;
            if bytes.first().is_some_and(|byte| byte & 0x80 != 0)
                || (bytes.first() == Some(&0) && (bytes.len() == 1 || bytes[1] & 0x80 == 0))
            {
                return Err(SshAgentCodecError::InvalidValue {
                    field,
                    reason: "not a canonical nonnegative mpint",
                });
            }
            let magnitude = bytes.strip_prefix(&[0]).unwrap_or(bytes);
            if magnitude.len() > 32 {
                return Err(SshAgentCodecError::InvalidValue {
                    field,
                    reason: "P-256 component exceeds 32 bytes",
                });
            }
            Ok(magnitude.to_vec())
        }
        let mut reader = SshReader::new(encoded);
        let r = read_component(&mut reader, "signature r")?;
        let s = read_component(&mut reader, "signature s")?;
        if !reader.remaining().is_empty() {
            return Err(SshAgentCodecError::TrailingData {
                trailing_bytes: reader.remaining().len(),
            });
        }
        Ok(Self { r, s })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = SshWriter::new();
        writer.write_mpint(&self.r);
        writer.write_mpint(&self.s);
        writer.into_bytes()
    }
    pub fn into_signature(self) -> SshSignature {
        SshSignature::Ecdsa(self)
    }
}
