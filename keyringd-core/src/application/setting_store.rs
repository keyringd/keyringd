use std::sync::{Arc, Mutex, MutexGuard};

use serde::{Serialize, de::DeserializeOwned};

use crate::persistence::{FileSaveError, SettingDocument, SettingRepository};

/// Owns one key-value document. Every read-modify-save uses the same lock.
pub struct SettingStore {
    document: Mutex<SettingDocument>,
    repository: Arc<dyn SettingRepository>,
}

impl SettingStore {
    pub fn new(repository: Arc<dyn SettingRepository>) -> std::io::Result<Self> {
        Ok(Self {
            document: Mutex::new(repository.load()?),
            repository,
        })
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> std::io::Result<Option<T>> {
        self.lock()?
            .values
            .get(key)
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    }

    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<(), FileSaveError> {
        let value = serde_json::to_value(value).map_err(std::io::Error::other)?;
        self.update(|document| {
            document.values.insert(key.to_owned(), value);
        })
    }

    pub fn remove(&self, key: &str) -> Result<(), FileSaveError> {
        self.update(|document| {
            document.values.remove(key);
        })
    }

    fn update(&self, update: impl FnOnce(&mut SettingDocument)) -> Result<(), FileSaveError> {
        let mut current = self.lock()?;
        let mut next = current.clone();
        update(&mut next);
        let result = self.repository.save(&next);
        if result.is_ok() || matches!(result, Err(FileSaveError::AfterReplace(_))) {
            // After replacement, disk readers already observe the new document.
            *current = next;
        }
        result
    }

    fn lock(&self) -> std::io::Result<MutexGuard<'_, SettingDocument>> {
        self.document
            .lock()
            .map_err(|_| std::io::Error::other("setting store lock was poisoned"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::MockSettingRepository;

    #[test]
    fn preserves_other_keys_and_distinguishes_absence_from_invalid_type() {
        let mut repository = MockSettingRepository::new();
        repository
            .expect_load()
            .returning(|| Ok(SettingDocument::default()));
        let mut sequence = mockall::Sequence::new();
        repository
            .expect_save()
            .times(1)
            .in_sequence(&mut sequence)
            .withf(|doc| {
                doc.values.len() == 1 && doc.values.get("first") == Some(&serde_json::json!(true))
            })
            .returning(|_| Ok(()));
        repository
            .expect_save()
            .times(1)
            .in_sequence(&mut sequence)
            .withf(|doc| {
                doc.values.len() == 2
                    && doc.values.get("first") == Some(&serde_json::json!(true))
                    && doc.values.get("second") == Some(&serde_json::json!("value"))
            })
            .returning(|_| Ok(()));
        repository
            .expect_save()
            .times(1)
            .in_sequence(&mut sequence)
            .withf(|doc| {
                doc.values.len() == 1
                    && doc.values.get("second") == Some(&serde_json::json!("value"))
            })
            .returning(|_| Ok(()));
        let store = SettingStore::new(Arc::new(repository)).unwrap();
        assert_eq!(store.get::<bool>("first").unwrap(), None);
        store.set("first", &true).unwrap();
        store.set("second", &"value").unwrap();
        assert_eq!(store.get::<bool>("first").unwrap(), Some(true));
        assert_eq!(
            store.get::<bool>("second").unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        store.remove("first").unwrap();
        assert_eq!(store.get::<bool>("first").unwrap(), None);
        assert_eq!(
            store.get::<String>("second").unwrap().as_deref(),
            Some("value")
        );
    }

    #[test]
    fn memory_follows_whether_the_file_was_replaced_even_when_save_fails() {
        for replaced in [false, true] {
            let mut repository = MockSettingRepository::new();
            repository
                .expect_load()
                .returning(|| Ok(SettingDocument::default()));
            repository.expect_save().times(1).returning(move |_| {
                let error = std::io::Error::other("save failed");
                Err(if replaced {
                    FileSaveError::AfterReplace(error)
                } else {
                    FileSaveError::BeforeReplace(error)
                })
            });
            let store = SettingStore::new(Arc::new(repository)).unwrap();
            assert!(store.set("selected", &true).is_err());
            assert_eq!(
                store.get::<bool>("selected").unwrap(),
                replaced.then_some(true)
            );
        }
    }

    #[test]
    fn concurrent_updates_do_not_discard_other_keys() {
        let mut repository = MockSettingRepository::new();
        repository
            .expect_load()
            .returning(|| Ok(SettingDocument::default()));
        repository.expect_save().times(16).returning(|_| Ok(()));
        let store = Arc::new(SettingStore::new(Arc::new(repository)).unwrap());
        std::thread::scope(|scope| {
            for index in 0..16 {
                let store = store.clone();
                scope.spawn(move || store.set(&index.to_string(), &index).unwrap());
            }
        });
        for index in 0..16 {
            assert_eq!(store.get::<i32>(&index.to_string()).unwrap(), Some(index));
        }
    }
}
