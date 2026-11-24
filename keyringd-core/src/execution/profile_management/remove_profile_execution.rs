use crate::application::ApplicationError;
use crate::application::ProfileStore;
use crate::execution::ExecutionScope;
use crate::execution::util::delete_key_best_effort;
use crate::model::ProfileId;
use crate::service::DeviceKeyService;

pub(crate) struct RemoveProfileExecution<'a, D: DeviceKeyService> {
    scope: ExecutionScope,
    device_key_service: &'a D,
    profiles: &'a ProfileStore,
}

impl<'a, D: DeviceKeyService> RemoveProfileExecution<'a, D> {
    pub(crate) fn new(device_key_service: &'a D, profiles: &'a ProfileStore) -> Self {
        Self {
            scope: ExecutionScope::new("remove_profile"),
            device_key_service,
            profiles,
        }
    }
}

impl<D> RemoveProfileExecution<'_, D>
where
    D: DeviceKeyService,
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
        let removed = update.remove_profile(profile_id)?;
        update.commit()?;
        delete_key_best_effort(
            self.device_key_service,
            context,
            removed.device_key().handle(),
        )
        .await;
        Ok(())
    }
}
