use crate::{SshAgentCodecError, SshAgentRequest};
pub(super) type IncomingRequest = Result<SshAgentRequest, SshAgentCodecError>;
