use crate::execution::ExecutionScope;
use crate::service::DeviceKeyMaterial;
use crate::service::DeviceKeyProtectionChangeResult;
use crate::signing::fido::PasskeyAssertionProvider;

use crate::application::{ApplicationError, ProfileStore};
use crate::execution::util::DeviceKeyCertificateIssuer;
use crate::execution::util::delete_key_best_effort;
use crate::model::ProfileId;
use crate::model::device_key::DeviceKey;
use crate::model::device_key::DeviceKeyAlgorithm;
use crate::service::DeviceKeyProtectionChange;
use crate::service::DeviceKeyProtectionChangeRequest;
use crate::service::DeviceKeyService;
use crate::service::{PasskeyService, SerializedPasskeyService};

pub(crate) struct ChangeDeviceKeyProtectionExecution<'a, D: DeviceKeyService, F> {
    scope: ExecutionScope,
    device_key_service: &'a D,
    passkey_service: &'a SerializedPasskeyService<F>,
    profiles: &'a ProfileStore,
}

impl<'a, D: DeviceKeyService, F> ChangeDeviceKeyProtectionExecution<'a, D, F> {
    pub(crate) fn new(
        device_key_service: &'a D,
        passkey_service: &'a SerializedPasskeyService<F>,
        profiles: &'a ProfileStore,
    ) -> Self {
        Self {
            scope: ExecutionScope::new("change_device_key_protection"),
            device_key_service,
            passkey_service,
            profiles,
        }
    }
}

impl<D, F> ChangeDeviceKeyProtectionExecution<'_, D, F>
where
    D: DeviceKeyService,
    F: PasskeyService,
{
    pub(crate) async fn execute(
        self,
        context: &D::OperationContext,
        profile_id: &ProfileId,
        change: DeviceKeyProtectionChange,
    ) -> Result<(), ApplicationError> {
        self.scope
            .run(self.execute_inner(context, profile_id, change))
            .await
    }

    async fn execute_inner(
        self,
        context: &D::OperationContext,
        profile_id: &ProfileId,
        change: DeviceKeyProtectionChange,
    ) -> Result<(), ApplicationError> {
        let mut update = self.profiles.begin_update().await;
        let profile = update.profile_mut(profile_id)?;
        if profile.device_key().local_authentication() != change.expected_current_authentication() {
            return Err(ApplicationError::InvariantViolation(
                "Local authentication change does not match the current state".into(),
            ));
        }
        let DeviceKeyProtectionChangeResult {
            updated_key: DeviceKeyMaterial { handle, public_key },
            delete_after_commit,
            delete_on_failure,
        } = self
            .device_key_service
            .change_key_protection(
                context,
                DeviceKeyProtectionChangeRequest {
                    profile_name: profile.display_name(),
                    handle: profile.device_key().handle(),
                    change,
                    algorithm: DeviceKeyAlgorithm::EcdsaP256,
                },
            )
            .await
            .map_err(|error| {
                ApplicationError::from_device_key_error(self.device_key_service, error)
            })?;
        let result = async {
            let public_key_changed = public_key
                .to_ssh_public_key(profile.device_key().relying_party_id())
                .map_err(|error| ApplicationError::Certificate(error.into()))?
                != profile.device_key().certificate().public_key();
            let certificate = if !public_key_changed {
                profile.device_key().certificate().clone()
            } else {
                self.passkey_service
                    .run(|passkey_service| async {
                        let mut fido_target =
                            crate::execution::operation::FidoOperationTarget::Unselected;
                        DeviceKeyCertificateIssuer::new(
                            PasskeyAssertionProvider::new(passkey_service, &mut fido_target),
                            profile.passkey(),
                        )
                        .issue(&public_key, profile.device_key().relying_party_id())
                        .await
                    })
                    .await?
            };
            profile.replace_device_key(
                DeviceKey::new(
                    handle,
                    certificate,
                    change.requested(),
                    profile.device_key().signature_algorithm(),
                )
                .map_err(|error| ApplicationError::InvariantViolation(error.into()))?,
            );
            Ok(())
        }
        .await
        .and_then(|()| update.commit());
        if matches!(result, Err(ApplicationError::ProfileDurabilityUncertain(_))) {
            return result;
        }
        let unused_handle = if result.is_ok() {
            delete_after_commit
        } else {
            delete_on_failure
        };
        if let Some(handle) = unused_handle {
            delete_key_best_effort(self.device_key_service, context, &handle).await;
        }
        result
    }
}
