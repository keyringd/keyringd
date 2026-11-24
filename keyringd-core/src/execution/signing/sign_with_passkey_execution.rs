use super::signing_profile;
use crate::application::{ApplicationError, ProfileStore};
use crate::execution::ExecutionScope;
use crate::model::{ProfileId, ProfileSshIdentityKind};
use crate::service::{PasskeyService, SerializedPasskeyService};
use crate::signing::fido::PasskeyAssertionProvider;
use keyringd_ssh_agent_protocol::SshAgentSignRequest;

pub(crate) struct SignWithPasskeyExecution<'a, F> {
    scope: ExecutionScope,
    passkey_service: &'a SerializedPasskeyService<F>,
    profiles: &'a ProfileStore,
}

impl<'a, F> SignWithPasskeyExecution<'a, F> {
    pub(crate) fn new(
        passkey_service: &'a SerializedPasskeyService<F>,
        profiles: &'a ProfileStore,
    ) -> Self {
        Self {
            scope: ExecutionScope::new("sign_with_passkey"),
            passkey_service,
            profiles,
        }
    }
}

impl<F> SignWithPasskeyExecution<'_, F>
where
    F: PasskeyService,
{
    pub(crate) async fn execute(
        self,
        profile_id: &ProfileId,
        request: &SshAgentSignRequest,
    ) -> Result<keyringd_ssh_agent_protocol::SshSignature, ApplicationError> {
        self.scope
            .run(self.execute_inner(profile_id, request))
            .await
    }

    async fn execute_inner(
        self,
        profile_id: &ProfileId,
        request: &SshAgentSignRequest,
    ) -> Result<keyringd_ssh_agent_protocol::SshSignature, ApplicationError> {
        self.passkey_service
            .run(|passkey_service| async move {
                let profile = signing_profile(
                    self.profiles,
                    profile_id,
                    request,
                    &[ProfileSshIdentityKind::Passkey],
                )?;
                let mut fido_target = crate::execution::operation::FidoOperationTarget::Unselected;
                PasskeyAssertionProvider::new(passkey_service, &mut fido_target)
                    .sign_ssh(profile.passkey(), request.data())
                    .await
            })
            .await
    }
}
