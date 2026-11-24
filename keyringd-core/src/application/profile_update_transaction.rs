use super::{ApplicationError, OperationRejectionReason};
use crate::model::{Profile, ProfileId};
use crate::persistence::{
    PROFILES_SCHEMA_VERSION, ProfileRepository, ProfileSaveError, ProfilesDocument,
};
use tokio::sync::{MutexGuard, watch};

/// An isolated working copy whose update lock stays held through commit and cleanup.
/// Dropping it without committing leaves storage and the published snapshot unchanged.
pub(crate) struct ProfileUpdateTransaction<'a> {
    _guard: MutexGuard<'a, ()>,
    profiles: Vec<Profile>,
    published: &'a watch::Sender<Vec<Profile>>,
    repository: &'a dyn ProfileRepository,
}

impl<'a> ProfileUpdateTransaction<'a> {
    pub(super) fn new(
        guard: MutexGuard<'a, ()>,
        published: &'a watch::Sender<Vec<Profile>>,
        repository: &'a dyn ProfileRepository,
    ) -> Self {
        Self {
            _guard: guard,
            profiles: published.borrow().clone(),
            published,
            repository,
        }
    }

    pub(crate) fn add_profile(&mut self, profile: Profile) {
        self.profiles.push(profile);
    }

    pub(crate) fn profile_mut(
        &mut self,
        profile_id: &ProfileId,
    ) -> Result<&mut Profile, ApplicationError> {
        self.profiles
            .iter_mut()
            .find(|profile| profile.id() == profile_id)
            .ok_or(ApplicationError::NotFound("profile"))
    }

    pub(crate) fn remove_profile(
        &mut self,
        profile_id: &ProfileId,
    ) -> Result<Profile, ApplicationError> {
        let index = self
            .profiles
            .iter()
            .position(|profile| profile.id() == profile_id)
            .ok_or(ApplicationError::NotFound("profile"))?;
        Ok(self.profiles.remove(index))
    }

    pub(crate) fn reorder_profiles(
        &mut self,
        profile_ids: &[ProfileId],
    ) -> Result<bool, ApplicationError> {
        let current = &self.profiles;
        if profile_ids.len() != current.len() {
            return Err(ApplicationError::OperationRejected(
                OperationRejectionReason::ProfileOrderLength,
            ));
        }
        let mut reordered = Vec::with_capacity(current.len());
        for profile_id in profile_ids {
            let profile = current
                .iter()
                .find(|profile| profile.id() == profile_id)
                .cloned()
                .ok_or(ApplicationError::OperationRejected(
                    OperationRejectionReason::UnknownProfileInOrder,
                ))?;
            if reordered
                .iter()
                .any(|profile: &Profile| profile.id() == profile_id)
            {
                return Err(ApplicationError::OperationRejected(
                    OperationRejectionReason::DuplicateProfileInOrder,
                ));
            }
            reordered.push(profile);
        }
        let changed = current
            .iter()
            .map(Profile::id)
            .ne(reordered.iter().map(Profile::id));
        if changed {
            self.profiles = reordered;
        }
        Ok(changed)
    }

    pub(crate) fn commit(&self) -> Result<(), ApplicationError> {
        let document = ProfilesDocument {
            schema_version: PROFILES_SCHEMA_VERSION,
            profiles: self.profiles.clone(),
        };
        let result = match self.repository.save(&document) {
            Ok(()) => Ok(()),
            Err(ProfileSaveError::InvalidDocument(error)) => {
                return Err(ApplicationError::InvariantViolation(error.into()));
            }
            Err(ProfileSaveError::BeforeReplace(error)) => return Err(error.into()),
            Err(ProfileSaveError::AfterReplace(error)) => {
                Err(ApplicationError::ProfileDurabilityUncertain(error))
            }
        };
        // After rename, readers already see the new document even if its durability is uncertain.
        self.published.send_replace(document.profiles);
        result
    }
}
