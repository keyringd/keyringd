use keyringd_core::persistence::ProfileSaveError;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{OperationJournal, TestOperation};
use keyringd_core::model::Profile;
use keyringd_core::persistence::{ProfileRepository, ProfilesDocument};

pub struct MemoryProfileRepository {
    state: Mutex<ProfilesDocument>,
    fail_saves: AtomicBool,
    #[cfg(test)]
    fail_directory_sync: AtomicBool,
    journal: Option<OperationJournal>,
}

impl Default for MemoryProfileRepository {
    fn default() -> Self {
        Self {
            state: Mutex::new(ProfilesDocument::default()),
            fail_saves: AtomicBool::new(false),
            #[cfg(test)]
            fail_directory_sync: AtomicBool::new(false),
            journal: None,
        }
    }
}

impl MemoryProfileRepository {
    pub fn with_state(state: ProfilesDocument) -> Self {
        Self {
            state: Mutex::new(state),
            ..Self::default()
        }
    }

    #[cfg(test)]
    pub(crate) fn with_journal(journal: OperationJournal) -> Self {
        Self {
            state: Mutex::new(ProfilesDocument::default()),
            journal: Some(journal),
            ..Self::default()
        }
    }

    #[cfg(test)]
    pub(crate) fn fail_directory_sync(&self) {
        self.fail_directory_sync.store(true, Ordering::SeqCst);
    }

    pub fn snapshot(&self) -> Option<ProfilesDocument> {
        Some(self.state.lock().unwrap().clone())
    }

    pub fn set_save_failure(&self, fail: bool) {
        self.fail_saves.store(fail, Ordering::SeqCst);
    }
}

impl ProfileRepository for MemoryProfileRepository {
    fn load(&self) -> std::io::Result<Vec<Profile>> {
        let state = self.state.lock().unwrap();
        state
            .validate()
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        Ok(state.profiles.clone())
    }

    fn save(&self, document: &ProfilesDocument) -> Result<(), ProfileSaveError> {
        document.validate()?;
        if self.fail_saves.load(Ordering::SeqCst) {
            return Err(std::io::Error::other("injected profile save failure").into());
        }
        if let Some(journal) = &self.journal {
            journal.record(TestOperation::ProfilesSaved(
                document
                    .profiles
                    .iter()
                    .map(|profile| profile.device_key().handle().as_ref().to_vec())
                    .collect(),
            ));
        }
        *self.state.lock().unwrap() = document.clone();
        #[cfg(test)]
        if self.fail_directory_sync.load(Ordering::SeqCst) {
            return Err(ProfileSaveError::AfterReplace(std::io::Error::other(
                "injected directory sync failure",
            )));
        }
        Ok(())
    }
}
