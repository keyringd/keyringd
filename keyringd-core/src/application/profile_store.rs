use std::sync::Arc;

use super::{ApplicationError, ProfileUpdateTransaction};
use crate::model::{Profile, ProfileId};
use crate::persistence::ProfileRepository;

/// Owns the live Profile collection and commits every published change to storage first.
pub(crate) struct ProfileStore {
    profiles: tokio::sync::watch::Sender<Vec<Profile>>,
    repository: Arc<dyn ProfileRepository>,
    update_lock: tokio::sync::Mutex<()>,
}

impl ProfileStore {
    pub(crate) fn new(profiles: Vec<Profile>, repository: Arc<dyn ProfileRepository>) -> Self {
        let (profiles, _) = tokio::sync::watch::channel(profiles);
        Self {
            profiles,
            repository,
            update_lock: tokio::sync::Mutex::new(()),
        }
    }

    pub(crate) async fn begin_update(&self) -> ProfileUpdateTransaction<'_> {
        ProfileUpdateTransaction::new(
            self.update_lock.lock().await,
            &self.profiles,
            self.repository.as_ref(),
        )
    }

    pub(crate) fn try_begin_update(
        &self,
    ) -> Result<ProfileUpdateTransaction<'_>, ApplicationError> {
        let guard = self
            .update_lock
            .try_lock()
            .map_err(|_| ApplicationError::Conflict("another profile operation is in progress"))?;
        Ok(ProfileUpdateTransaction::new(
            guard,
            &self.profiles,
            self.repository.as_ref(),
        ))
    }

    pub(crate) fn snapshot(&self) -> Vec<Profile> {
        self.profiles.borrow().clone()
    }

    pub(crate) fn get(&self, profile_id: &ProfileId) -> Result<Profile, ApplicationError> {
        self.profiles
            .borrow()
            .iter()
            .find(|profile| profile.id() == profile_id)
            .cloned()
            .ok_or(ApplicationError::NotFound("profile"))
    }

    pub(crate) fn subscribe(&self) -> tokio::sync::watch::Receiver<Vec<Profile>> {
        self.profiles.subscribe()
    }
}
