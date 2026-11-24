use crate::execution::ssh_connection::TestSshConnectionExecution;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use super::{ApplicationError, ProfileStore};
use crate::application::ProfileSetupFailure;
use crate::application::ProfileSetupRequest;
use crate::application::{PasskeyCredentialSelector, ProfileSetupProgressReporter};
use crate::execution::key_management::{
    ChangeDeviceKeyProtectionExecution, ReplaceDeviceKeyExecution,
};
use crate::execution::profile_management::{
    RemoveProfileExecution, RenameProfileExecution, SetProfileEnabledExecution,
    SetProfileOrderExecution,
};
use crate::execution::profile_setup::SetupProfileExecution;
use crate::execution::signing::{SignWithDeviceKeyExecution, SignWithPasskeyExecution};
use crate::model::Profile;
use crate::model::ProfileId;
use crate::persistence::ProfileRepository;
use crate::service::DeviceKeyProtectionChange;
use crate::service::{DeviceKeyService, PasskeyService, SerializedPasskeyService};
use keyringd_ssh_agent_protocol::{SshAgentSignRequest, SshSignature};

pub struct KeyringdApplication<D, F>
where
    D: DeviceKeyService,
{
    device_key_service: D,
    passkey_service: SerializedPasskeyService<F>,
    profiles: ProfileStore,
}

impl<D, F> KeyringdApplication<D, F>
where
    D: DeviceKeyService + 'static,
    F: PasskeyService + 'static,
{
    pub fn new(
        key_service: D,
        passkey_service: F,
        repository: Arc<dyn ProfileRepository>,
    ) -> Result<Arc<Self>, ApplicationError> {
        let loaded_profiles = repository.load()?;
        let profiles = ProfileStore::new(loaded_profiles, repository);
        let application = Arc::new(Self {
            device_key_service: key_service,
            passkey_service: SerializedPasskeyService::new(passkey_service),
            profiles,
        });
        application.log_initial_state();
        Ok(application)
    }

    fn log_initial_state(&self) {
        tracing::info!(profiles = self.profiles().len(), "application initialized");
    }

    pub async fn check_device_key_availability(&self) -> Result<(), ApplicationError> {
        self.device_key_service
            .check_availability()
            .await
            .map_err(|error| ApplicationError::DeviceKeyUnavailable(Box::new(error)))
    }

    pub fn profiles(&self) -> Vec<Profile> {
        self.profiles.snapshot()
    }

    pub fn profile(&self, profile_id: &ProfileId) -> Result<Profile, ApplicationError> {
        self.profiles.get(profile_id)
    }

    /// Rechecks the current profile and selects its current identity before contacting OpenSSH.
    pub async fn test_ssh_connection<S: crate::service::OpenSshService + Sync>(
        &self,
        profile_id: &ProfileId,
        identity: crate::model::ProfileSshIdentityKind,
        target: String,
        open_ssh: &S,
    ) -> Result<crate::service::SshConnectionTestReport, ApplicationError> {
        TestSshConnectionExecution::new(&self.profiles, open_ssh)
            .execute(profile_id, identity, target)
            .await
    }

    pub fn subscribe_profile_changes(&self) -> tokio::sync::watch::Receiver<Vec<Profile>> {
        self.profiles.subscribe()
    }

    /// Reports progress separately from cancellation. Once the profile is saved,
    /// setup succeeds even if cancellation is requested immediately afterward.
    pub async fn setup_profile(
        &self,
        context: &D::OperationContext,
        cancellation: &CancellationToken,
        request: ProfileSetupRequest,
        progress_reporter: impl ProfileSetupProgressReporter,
        credential_selector: impl PasskeyCredentialSelector,
    ) -> Result<Profile, ProfileSetupFailure> {
        SetupProfileExecution::new(
            &self.device_key_service,
            &self.passkey_service,
            &self.profiles,
        )
        .execute(
            context,
            cancellation,
            request,
            progress_reporter,
            credential_selector,
        )
        .await
    }

    pub async fn remove_profile(
        &self,
        context: &D::OperationContext,
        profile_id: &ProfileId,
    ) -> Result<(), ApplicationError> {
        RemoveProfileExecution::new(&self.device_key_service, &self.profiles)
            .execute(context, profile_id)
            .await
    }

    pub fn rename_profile(
        &self,
        profile_id: &ProfileId,
        display_name: String,
    ) -> Result<(), ApplicationError> {
        RenameProfileExecution::new(&self.profiles).execute(profile_id, display_name)
    }

    pub fn set_profile_enabled(
        &self,
        profile_id: &ProfileId,
        enabled: bool,
    ) -> Result<(), ApplicationError> {
        SetProfileEnabledExecution::new(&self.profiles).execute(profile_id, enabled)
    }

    pub fn set_profile_order(&self, profile_ids: &[ProfileId]) -> Result<bool, ApplicationError> {
        SetProfileOrderExecution::new(&self.profiles).execute(profile_ids)
    }

    pub async fn change_device_key_protection(
        &self,
        context: &D::OperationContext,
        profile_id: &ProfileId,
        change: DeviceKeyProtectionChange,
    ) -> Result<(), ApplicationError> {
        ChangeDeviceKeyProtectionExecution::new(
            &self.device_key_service,
            &self.passkey_service,
            &self.profiles,
        )
        .execute(context, profile_id, change)
        .await
    }

    pub async fn replace_device_key(
        &self,
        context: &D::OperationContext,
        profile_id: &ProfileId,
    ) -> Result<(), ApplicationError> {
        ReplaceDeviceKeyExecution::new(
            &self.device_key_service,
            &self.passkey_service,
            &self.profiles,
        )
        .execute(context, profile_id)
        .await
    }

    pub(crate) async fn sign_with_device_key(
        &self,
        context: &D::OperationContext,
        profile_id: &ProfileId,
        request: &SshAgentSignRequest,
    ) -> Result<SshSignature, ApplicationError> {
        SignWithDeviceKeyExecution::new(&self.device_key_service, &self.profiles)
            .execute(context, profile_id, request)
            .await
    }

    pub(crate) async fn sign_with_passkey(
        &self,
        profile_id: &ProfileId,
        request: &SshAgentSignRequest,
    ) -> Result<SshSignature, ApplicationError> {
        SignWithPasskeyExecution::new(&self.passkey_service, &self.profiles)
            .execute(profile_id, request)
            .await
    }
}
