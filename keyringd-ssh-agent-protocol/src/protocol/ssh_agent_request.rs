use super::{SshAgentExtensionRequest, SshAgentSignRequest};
use crate::protocol::codec::{SshAgentCodecError, SshReader};

const SSH2_AGENTC_REQUEST_IDENTITIES: u8 = 11;
const SSH2_AGENTC_SIGN_REQUEST: u8 = 13;
const SSH2_AGENTC_EXTENSION: u8 = 27;

#[derive(Debug)]
pub enum SshAgentRequest {
    Identities,
    Sign(SshAgentSignRequest),
    Extension(SshAgentExtensionRequest),
    Unknown { message_type: u8, data: Vec<u8> },
}

impl SshAgentRequest {
    pub fn from_bytes(payload: &[u8]) -> Result<Self, SshAgentCodecError> {
        let mut reader = SshReader::new(payload);
        let message_type = reader
            .read_u8()
            .map_err(|_| SshAgentCodecError::EmptyMessage)?;

        match message_type {
            SSH2_AGENTC_REQUEST_IDENTITIES => {
                finish(reader)?;
                Ok(Self::Identities)
            }
            SSH2_AGENTC_SIGN_REQUEST => Ok(Self::Sign(SshAgentSignRequest::from_bytes(
                reader.remaining(),
            )?)),
            SSH2_AGENTC_EXTENSION => Ok(Self::Extension(SshAgentExtensionRequest::from_bytes(
                reader.remaining(),
            )?)),
            message_type => Ok(Self::Unknown {
                message_type,
                data: reader.remaining().to_vec(),
            }),
        }
    }
}

fn finish(reader: SshReader<'_>) -> Result<(), SshAgentCodecError> {
    let trailing_bytes = reader.remaining().len();
    reader
        .finish()
        .map_err(|_| SshAgentCodecError::TrailingData { trailing_bytes })
}
