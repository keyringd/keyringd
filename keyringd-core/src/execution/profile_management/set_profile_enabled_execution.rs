use crate::application::ApplicationError;
use crate::application::ProfileStore;
use crate::execution::ExecutionScope;
use crate::model::ProfileId;

pub(crate) struct SetProfileEnabledExecution<'a> {
    scope: ExecutionScope,
    profiles: &'a ProfileStore,
}
impl<'a> SetProfileEnabledExecution<'a> {
    pub(crate) fn new(profiles: &'a ProfileStore) -> Self {
        Self {
            scope: ExecutionScope::new("set_profile_enabled"),
            profiles,
        }
    }
    pub(crate) fn execute(
        self,
        profile_id: &ProfileId,
        enabled: bool,
    ) -> Result<(), ApplicationError> {
        self.scope.run_sync(|| {
            let mut update = self.profiles.try_begin_update()?;
            update.profile_mut(profile_id)?.set_enabled(enabled);
            update.commit()
        })
    }
}
