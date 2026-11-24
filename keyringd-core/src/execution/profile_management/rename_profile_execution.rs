use crate::application::ProfileStore;
use crate::application::{ApplicationError, OperationRejectionReason};
use crate::execution::ExecutionScope;
use crate::model::ProfileId;

pub(crate) struct RenameProfileExecution<'a> {
    scope: ExecutionScope,
    profiles: &'a ProfileStore,
}
impl<'a> RenameProfileExecution<'a> {
    pub(crate) fn new(profiles: &'a ProfileStore) -> Self {
        Self {
            scope: ExecutionScope::new("rename_profile"),
            profiles,
        }
    }
    pub(crate) fn execute(
        self,
        profile_id: &ProfileId,
        name: String,
    ) -> Result<(), ApplicationError> {
        self.scope.run_sync(|| {
            let mut update = self.profiles.try_begin_update()?;
            update
                .profile_mut(profile_id)?
                .rename(name.trim().to_owned())
                .map_err(|_| OperationRejectionReason::InvalidProfileName)?;
            update.commit()
        })
    }
}
