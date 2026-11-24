use std::sync::Arc;

use super::{AtomicFileStorage, ProfileRepository, ProfileSaveError, ProfilesDocument};
use crate::model::Profile;

pub struct FileProfileRepository {
    file: Arc<dyn AtomicFileStorage>,
}

impl FileProfileRepository {
    pub fn new(file: Arc<dyn AtomicFileStorage>) -> Self {
        Self { file }
    }
}

impl ProfileRepository for FileProfileRepository {
    fn load(&self) -> std::io::Result<Vec<Profile>> {
        match self.file.read()? {
            Some(value) => Ok(ProfilesDocument::from_json(&value)?.profiles),
            None => Ok(Vec::new()),
        }
    }

    fn save(&self, document: &ProfilesDocument) -> Result<(), ProfileSaveError> {
        self.file.replace(&document.to_json()?).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{FileSaveError, MockAtomicFileStorage};

    #[test]
    fn missing_file_is_empty_but_read_errors_are_reported() {
        let mut file = MockAtomicFileStorage::new();
        file.expect_read().times(1).returning(|| Ok(None));
        let repository = FileProfileRepository::new(Arc::new(file));
        assert!(repository.load().unwrap().is_empty());

        let mut file = MockAtomicFileStorage::new();
        file.expect_read()
            .times(1)
            .returning(|| Err(std::io::ErrorKind::PermissionDenied.into()));
        let repository = FileProfileRepository::new(Arc::new(file));
        assert_eq!(
            repository.load().err().unwrap().kind(),
            std::io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn corrupt_documents_are_reported_without_writing() {
        for bytes in [
            b"not JSON".to_vec(),
            br#"{"schemaVersion":0,"profiles":[]}"#.to_vec(),
        ] {
            let mut file = MockAtomicFileStorage::new();
            file.expect_read().times(1).return_once(|| Ok(Some(bytes)));
            file.expect_replace().never();
            let repository = FileProfileRepository::new(Arc::new(file));
            assert_eq!(
                repository.load().err().unwrap().kind(),
                std::io::ErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn invalid_documents_are_rejected_before_file_io() {
        let mut file = MockAtomicFileStorage::new();
        file.expect_replace().never();
        let repository = FileProfileRepository::new(Arc::new(file));
        let document = ProfilesDocument {
            schema_version: 0,
            ..ProfilesDocument::default()
        };
        assert!(matches!(
            repository.save(&document),
            Err(ProfileSaveError::InvalidDocument(_))
        ));
    }

    #[test]
    fn preserves_the_file_replacement_failure_stage() {
        for replaced in [false, true] {
            let mut file = MockAtomicFileStorage::new();
            file.expect_replace()
                .times(1)
                .withf(|bytes| {
                    ProfilesDocument::from_json(bytes)
                        .is_ok_and(|document| document.profiles.is_empty())
                })
                .return_once(move |_| {
                    let error = std::io::Error::other("save failed");
                    Err(if replaced {
                        FileSaveError::AfterReplace(error)
                    } else {
                        FileSaveError::BeforeReplace(error)
                    })
                });
            let repository = FileProfileRepository::new(Arc::new(file));
            let error = repository.save(&ProfilesDocument::default()).unwrap_err();
            assert!(matches!(
                (replaced, error),
                (true, ProfileSaveError::AfterReplace(_))
                    | (false, ProfileSaveError::BeforeReplace(_))
            ));
        }
    }
}
