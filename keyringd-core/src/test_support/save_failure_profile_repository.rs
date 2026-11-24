use keyringd_core::persistence::ProfileSaveError;
use keyringd_core::persistence::ProfilesDocument;
use std::sync::atomic::{AtomicBool, Ordering};

use keyringd_core::model::Profile;
use keyringd_core::persistence::ProfileRepository;

/// Delegates persistence to the supplied repository, with an optional save failure
/// for UI retry tests. Reads always use the persisted product data.
pub struct SaveFailureProfileRepository<S> {
    inner: S,
    fail_saves: AtomicBool,
}

impl<S> SaveFailureProfileRepository<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            fail_saves: AtomicBool::new(false),
        }
    }

    pub fn set_save_failure(&self, fail: bool) {
        self.fail_saves.store(fail, Ordering::SeqCst);
    }
}

impl<S: ProfileRepository> ProfileRepository for SaveFailureProfileRepository<S> {
    fn load(&self) -> std::io::Result<Vec<Profile>> {
        self.inner.load()
    }

    fn save(&self, document: &ProfilesDocument) -> Result<(), ProfileSaveError> {
        if self.fail_saves.load(Ordering::SeqCst) {
            return Err(std::io::Error::other("injected profile save failure").into());
        }
        self.inner.save(document)
    }
}
