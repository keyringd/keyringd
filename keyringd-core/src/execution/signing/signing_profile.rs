use crate::application::{ApplicationError, OperationRejectionReason, ProfileStore};
use crate::model::{Profile, ProfileId, ProfileSshIdentity, ProfileSshIdentityKind};
use keyringd_ssh_agent_protocol::{SshAgentSignRequest, SshSigningPayload};

pub(in crate::execution) fn signing_profile(
    profiles: &ProfileStore,
    profile_id: &ProfileId,
    request: &SshAgentSignRequest,
    identity_kinds: &[ProfileSshIdentityKind],
) -> Result<Profile, ApplicationError> {
    let profile = profiles.get(profile_id)?;
    if !profile.is_enabled() {
        return Err(OperationRejectionReason::ProfileDisabled.into());
    }
    let now = std::time::SystemTime::now();
    let identities = identity_kinds
        .iter()
        .filter_map(|kind| ProfileSshIdentity::from_profile_at(&profile, *kind, now))
        .collect::<Vec<_>>();
    let matches_identity = |key| identities.iter().any(|identity| identity.key_blob() == key);
    if !matches_identity(request.key_blob()) {
        return Err(OperationRejectionReason::SshIdentityUnavailable.into());
    }
    if request.flags().bits() != 0 {
        return Err(OperationRejectionReason::InvalidSshSigningRequest.into());
    }
    let payload = SshSigningPayload::from_bytes(request.data())
        .map_err(|_| OperationRejectionReason::InvalidSshSigningRequest)?;
    if let SshSigningPayload::UserAuthentication(userauth) = payload
        && !matches_identity(userauth.key_blob())
    {
        return Err(OperationRejectionReason::InvalidSshSigningRequest.into());
    }
    Ok(profile)
}
