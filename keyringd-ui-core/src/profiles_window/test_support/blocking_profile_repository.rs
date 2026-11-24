use keyringd_core::model::Profile;
use keyringd_core::persistence::ProfileSaveError;
use keyringd_core::persistence::{ProfileRepository, ProfilesDocument};
use keyringd_core::test_support::MemoryProfileRepository;
use std::sync::{Mutex, mpsc};
use std::thread::ThreadId;
use tokio::sync::oneshot;

/// Blocks the first save until the test releases it, reporting the saving thread.
pub(crate) struct BlockingProfileRepository {
    inner: MemoryProfileRepository,
    entered: Mutex<Option<oneshot::Sender<ThreadId>>>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl BlockingProfileRepository {
    pub(crate) fn new(
        document: ProfilesDocument,
    ) -> (Self, oneshot::Receiver<ThreadId>, mpsc::Sender<()>) {
        let (entered, entry) = oneshot::channel();
        let (resume, release) = mpsc::channel();
        (
            Self {
                inner: MemoryProfileRepository::with_state(document),
                entered: Mutex::new(Some(entered)),
                release: Mutex::new(release),
            },
            entry,
            resume,
        )
    }
}

impl ProfileRepository for BlockingProfileRepository {
    fn load(&self) -> std::io::Result<Vec<Profile>> {
        self.inner.load()
    }

    fn save(&self, document: &ProfilesDocument) -> Result<(), ProfileSaveError> {
        if let Some(entered) = self.entered.lock().unwrap().take() {
            let _ = entered.send(std::thread::current().id());
            self.release
                .lock()
                .unwrap()
                .recv()
                .map_err(std::io::Error::other)?;
        }
        self.inner.save(document)
    }
}
