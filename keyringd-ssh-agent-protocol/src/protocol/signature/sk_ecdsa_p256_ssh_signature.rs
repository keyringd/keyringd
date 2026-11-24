use crate::protocol::codec::{SshAgentCodecError, SshReader, SshWriter};
use crate::{EcdsaP256SshSignature, SshSignature};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkEcdsaP256SshSignature {
    signature: EcdsaP256SshSignature,
    flags: u8,
    counter: u32,
}

impl SkEcdsaP256SshSignature {
    pub fn new(signature: EcdsaP256SshSignature, flags: u8, counter: u32) -> Self {
        Self {
            signature,
            flags,
            counter,
        }
    }

    pub fn signature(&self) -> &EcdsaP256SshSignature {
        &self.signature
    }

    pub fn flags(&self) -> u8 {
        self.flags
    }

    pub fn counter(&self) -> u32 {
        self.counter
    }

    pub fn from_bytes(encoded: &[u8]) -> Result<Self, SshAgentCodecError> {
        let mut reader = SshReader::new(encoded);
        let ecdsa = reader
            .read_string()
            .map_err(|_| SshAgentCodecError::Truncated {
                field: "ECDSA signature",
            })?;
        let signature = EcdsaP256SshSignature::from_bytes(ecdsa)?;
        let flags = reader
            .read_u8()
            .map_err(|_| SshAgentCodecError::Truncated {
                field: "signature flags",
            })?;
        let counter = reader
            .read_u32()
            .map_err(|_| SshAgentCodecError::Truncated {
                field: "signature counter",
            })?;
        if !reader.remaining().is_empty() {
            return Err(SshAgentCodecError::TrailingData {
                trailing_bytes: reader.remaining().len(),
            });
        }
        Ok(Self {
            signature,
            flags,
            counter,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = SshWriter::new();
        writer.write_string(&self.signature.to_bytes());
        writer.write_u8(self.flags);
        writer.write_u32(self.counter);
        writer.into_bytes()
    }
    pub fn into_signature(self) -> SshSignature {
        SshSignature::SkEcdsa(self)
    }
}
