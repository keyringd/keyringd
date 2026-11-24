use crate::application::{ApplicationError, OperationRejectionReason, ProfileStore};
use crate::execution::ExecutionScope;
use crate::model::{ProfileId, ProfileSshIdentity, ProfileSshIdentityKind};
use crate::service::{OpenSshService, SshConnectionTestReport};

pub(crate) struct TestSshConnectionExecution<'a, S> {
    scope: ExecutionScope,
    profiles: &'a ProfileStore,
    open_ssh: &'a S,
}

impl<'a, S: OpenSshService + Sync> TestSshConnectionExecution<'a, S> {
    pub(crate) fn new(profiles: &'a ProfileStore, open_ssh: &'a S) -> Self {
        Self {
            scope: ExecutionScope::new("test_ssh_connection"),
            profiles,
            open_ssh,
        }
    }

    /// Selects the current profile's identity immediately before contacting OpenSSH.
    pub(crate) async fn execute(
        self,
        profile_id: &ProfileId,
        identity: ProfileSshIdentityKind,
        target: String,
    ) -> Result<SshConnectionTestReport, ApplicationError> {
        self.scope
            .run(self.execute_inner(profile_id, identity, target))
            .await
    }

    async fn execute_inner(
        self,
        profile_id: &ProfileId,
        identity: ProfileSshIdentityKind,
        target: String,
    ) -> Result<SshConnectionTestReport, ApplicationError> {
        let profile = self.profiles.get(profile_id)?;
        if !profile.is_enabled() {
            return Err(OperationRejectionReason::ProfileDisabled.into());
        }
        let public_identity =
            ProfileSshIdentity::from_profile_at(&profile, identity, std::time::SystemTime::now())
                .ok_or(OperationRejectionReason::SshIdentityUnavailable)?;
        let result = self
            .open_ssh
            .test_connection(target, public_identity.to_public_key_line())
            .await;
        Ok(SshConnectionTestReport::new(
            &profile,
            &public_identity,
            result,
        ))
    }
}
