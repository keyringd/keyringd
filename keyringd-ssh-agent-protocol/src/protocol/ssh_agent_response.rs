use super::SshAgentIdentity;
use crate::SshSignature;
use crate::protocol::codec::{SshAgentCodecError, SshWriter};

const SSH2_AGENT_FAILURE: u8 = 5;
const SSH2_AGENT_SUCCESS: u8 = 6;
const SSH2_AGENT_IDENTITIES_ANSWER: u8 = 12;
const SSH2_AGENT_SIGN_RESPONSE: u8 = 14;
const SSH_AGENT_EXTENSION_FAILURE: u8 = 28;

#[derive(Debug)]
pub enum SshAgentResponse {
    Failure,
    ExtensionFailure,
    Success,
    IdentitiesAnswer { identities: Vec<SshAgentIdentity> },
    Signature { signature: SshSignature },
}

impl SshAgentResponse {
    pub fn to_bytes(&self) -> Result<Vec<u8>, SshAgentCodecError> {
        Ok(match self {
            Self::Failure => vec![SSH2_AGENT_FAILURE],
            Self::ExtensionFailure => vec![SSH_AGENT_EXTENSION_FAILURE],
            Self::Success => vec![SSH2_AGENT_SUCCESS],
            Self::IdentitiesAnswer { identities } => {
                let mut writer = SshWriter::new();
                writer.write_u8(SSH2_AGENT_IDENTITIES_ANSWER);
                let count = u32::try_from(identities.len())
                    .map_err(|_| SshAgentCodecError::TooManyIdentities)?;
                writer.write_u32(count);
                for identity in identities {
                    writer.write_bytes(&identity.to_bytes());
                }
                writer.into_bytes()
            }
            Self::Signature { signature } => {
                let mut writer = SshWriter::new();
                writer.write_u8(SSH2_AGENT_SIGN_RESPONSE);
                writer.write_string(&signature.to_bytes());
                writer.into_bytes()
            }
        })
    }
}
