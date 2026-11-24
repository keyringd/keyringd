use crate::application::ApplicationError;
use keyringd_ssh_agent_protocol::SshSignature;

/// Internal cryptographic primitive. Agent payload authorization belongs to the caller.
pub(crate) trait SshSigner: Send {
    fn sign(
        &mut self,
        message: &[u8],
    ) -> impl Future<Output = Result<SshSignature, ApplicationError>> + Send;
}
