use crate::application::ApplicationError;
use crate::application::ProfileStore;
use crate::model::ProfileId;
use crate::persistence::ProfilesDocument;
use crate::test_support::{MemoryProfileRepository, ModelProfileFixture};
use std::sync::Arc;

fn profile_store() -> (ProfileStore, Arc<MemoryProfileRepository>, ProfileId) {
    let mut profile = ModelProfileFixture::default().build();
    profile.rename("Original".into()).unwrap();
    let repository = Arc::new(MemoryProfileRepository::with_state(ProfilesDocument {
        profiles: vec![profile.clone()],
        ..Default::default()
    }));
    let id = profile.id().clone();
    (
        ProfileStore::new(vec![profile], repository.clone()),
        repository,
        id,
    )
}

keyringd_testkit::scenario! {
    group "ProfileStore::begin_update" {
        case "Serializes updates through transaction drop and starts the next update from committed state" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "commit publishes the new state while the next writer waits for the transaction to be dropped" {
                let (profile_store, repository, id) = profile_store();
                let mut changes = profile_store.subscribe();
                let mut first = profile_store.begin_update().await;
                first.profile_mut(&id).unwrap().rename("Renamed".into()).unwrap();
                assert_eq!(profile_store.get(&id).unwrap().display_name(), "Original");
                assert!(!changes.has_changed().unwrap());

                let waiting = profile_store.begin_update();
                tokio::pin!(waiting);
                assert!(futures_util::poll!(waiting.as_mut()).is_pending());
                first.commit().unwrap();
                assert!(matches!(
                    profile_store.try_begin_update(),
                    Err(ApplicationError::Conflict(_))
                ));
                assert!(changes.has_changed().unwrap());
                assert_eq!(changes.borrow_and_update()[0].display_name(), "Renamed");
                drop(first);

                let mut second = waiting.await;
                assert_eq!(second.profile_mut(&id).unwrap().display_name(), "Renamed");
                second.profile_mut(&id).unwrap().set_enabled(false);
                second.commit().unwrap();
                drop(second);

                let saved = repository.snapshot().unwrap();
                assert_eq!(saved.profiles[0].display_name(), "Renamed");
                assert!(!saved.profiles[0].is_enabled());
                assert_eq!(profile_store.get(&id).unwrap().display_name(), "Renamed");
                assert!(!profile_store.get(&id).unwrap().is_enabled());
            }
        }
    }
    group "ProfileUpdateTransaction::commit" {
        case "Discards uncommitted changes and leaves storage and subscribers unchanged after a failed commit" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "discarded and unsaved drafts preserve stored and published profiles and release the update lock" {
                let (profile_store, repository, id) = profile_store();
                let changes = profile_store.subscribe();
                {
                    let mut update = profile_store.begin_update().await;
                    update.profile_mut(&id).unwrap().rename("Discarded".into()).unwrap();
                }
                assert_eq!(profile_store.get(&id).unwrap().display_name(), "Original");
                assert_eq!(
                    repository.snapshot().unwrap().profiles[0].display_name(),
                    "Original"
                );
                assert!(!changes.has_changed().unwrap());

                repository.set_save_failure(true);
                {
                    let mut update = profile_store.try_begin_update().unwrap();
                    update.profile_mut(&id).unwrap().rename("Unsaved".into()).unwrap();
                    assert!(matches!(
                        update.commit(),
                        Err(ApplicationError::ProfileRepository(_))
                    ));
                }
                assert_eq!(profile_store.get(&id).unwrap().display_name(), "Original");
                assert_eq!(
                    repository.snapshot().unwrap().profiles[0].display_name(),
                    "Original"
                );
                assert!(!changes.has_changed().unwrap());
                assert!(profile_store.try_begin_update().is_ok());
            }
        }
    }
}
