use crate::application::ApplicationError;
use crate::application::ProfileStore;
use crate::execution::ExecutionScope;
use crate::model::ProfileId;

pub(crate) struct SetProfileOrderExecution<'a> {
    scope: ExecutionScope,
    profiles: &'a ProfileStore,
}
impl<'a> SetProfileOrderExecution<'a> {
    pub(crate) fn new(profiles: &'a ProfileStore) -> Self {
        Self {
            scope: ExecutionScope::new("set_profile_order"),
            profiles,
        }
    }
    pub(crate) fn execute(self, profile_ids: &[ProfileId]) -> Result<bool, ApplicationError> {
        self.scope.run_sync(|| {
            let mut update = self.profiles.try_begin_update()?;
            let changed = update.reorder_profiles(profile_ids)?;
            if changed {
                update.commit()?;
            }
            Ok(changed)
        })
    }
}
