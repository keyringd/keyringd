use super::{SshSigningPayloadError, SshUserauthSigningPayload, SshsigSigningPayload};

const SSHSIG_MAGIC: &[u8] = b"SSHSIG";

/// A recognized SSH-specific payload carried by `SSH_AGENTC_SIGN_REQUEST`.
///
/// SSH Agent treats request data as opaque bytes. This parser is for agents
/// backed by web-origin security keys that must reject arbitrary WebAuthn
/// challenges while accepting SSH user authentication and SSHSIG operations.
#[derive(Debug)]
pub enum SshSigningPayload<'a> {
    UserAuthentication(SshUserauthSigningPayload<'a>),
    Sshsig(SshsigSigningPayload<'a>),
}

impl<'a> SshSigningPayload<'a> {
    pub fn from_bytes(data: &'a [u8]) -> Result<Self, SshSigningPayloadError> {
        if data.starts_with(SSHSIG_MAGIC) {
            SshsigSigningPayload::from_bytes(data).map(Self::Sshsig)
        } else {
            SshUserauthSigningPayload::from_bytes(data)
                .map(Self::UserAuthentication)
                .map_err(|_| SshSigningPayloadError::Unsupported)
        }
    }
}
