use crate::execution::ExecutionScope;
use crate::service::DeviceKeyMaterial;
use crate::signing::fido::PasskeyAssertionProvider;

use crate::application::{ApplicationError, ProfileStore};
use crate::execution::util::DeviceKeyCertificateIssuer;
use crate::execution::util::delete_key_best_effort;
use crate::model::ProfileId;
use crate::model::device_key::DeviceKey;
use crate::model::device_key::DeviceKeyAlgorithm;
use crate::service::DeviceKeyGenerationReason;
use crate::service::DeviceKeyGenerationRequest;
use crate::service::DeviceKeyService;
use crate::service::{PasskeyService, SerializedPasskeyService};

pub(crate) struct ReplaceDeviceKeyExecution<'a, D: DeviceKeyService, F> {
    scope: ExecutionScope,
    device_key_service: &'a D,
    passkey_service: &'a SerializedPasskeyService<F>,
    profiles: &'a ProfileStore,
}

impl<'a, D: DeviceKeyService, F> ReplaceDeviceKeyExecution<'a, D, F> {
    pub(crate) fn new(
        device_key_service: &'a D,
        passkey_service: &'a SerializedPasskeyService<F>,
        profiles: &'a ProfileStore,
    ) -> Self {
        Self {
            scope: ExecutionScope::new("replace_device_key"),
            device_key_service,
            passkey_service,
            profiles,
        }
    }
}

impl<D, F> ReplaceDeviceKeyExecution<'_, D, F>
where
    D: DeviceKeyService,
    F: PasskeyService,
{
    pub(crate) async fn execute(
        self,
        context: &D::OperationContext,
        profile_id: &ProfileId,
    ) -> Result<(), ApplicationError> {
        self.scope
            .run(self.execute_inner(context, profile_id))
            .await
    }

    async fn execute_inner(
        self,
        context: &D::OperationContext,
        profile_id: &ProfileId,
    ) -> Result<(), ApplicationError> {
        let mut update = self.profiles.begin_update().await;
        let profile = update.profile_mut(profile_id)?;
        let DeviceKeyMaterial { handle, public_key } = self
            .device_key_service
            .generate_key(
                context,
                DeviceKeyGenerationRequest {
                    profile_name: profile.display_name(),
                    reason: DeviceKeyGenerationReason::Replacement,
                    algorithm: DeviceKeyAlgorithm::EcdsaP256,
                    local_authentication: profile.device_key().local_authentication(),
                },
            )
            .await
            .map_err(|error| {
                ApplicationError::from_device_key_error(self.device_key_service, error)
            })?;
        let obsolete_handle = profile.device_key().handle().clone();
        let mut fido_target = crate::execution::operation::FidoOperationTarget::Unselected;
        let result = async {
            let certificate = self
                .passkey_service
                .run(|passkey_service| async {
                    DeviceKeyCertificateIssuer::new(
                        PasskeyAssertionProvider::new(passkey_service, &mut fido_target),
                        profile.passkey(),
                    )
                    .issue(&public_key, profile.device_key().relying_party_id())
                    .await
                })
                .await?;
            let device_key = DeviceKey::new(
                handle.clone(),
                certificate,
                profile.device_key().local_authentication(),
                profile.device_key().signature_algorithm(),
            )
            .map_err(|error| ApplicationError::InvariantViolation(error.into()))?;
            profile.replace_device_key(device_key);
            Ok(())
        }
        .await
        .and_then(|()| update.commit());
        if matches!(result, Err(ApplicationError::ProfileDurabilityUncertain(_))) {
            return result;
        }
        let unused_handle = if result.is_ok() {
            &obsolete_handle
        } else {
            &handle
        };
        delete_key_best_effort(self.device_key_service, context, unused_handle).await;
        if result.is_ok() {
            tracing::debug!(
                profile_id = profile_id.as_str(),
                "obsolete device key cleanup attempted"
            );
        }
        result
    }
}
