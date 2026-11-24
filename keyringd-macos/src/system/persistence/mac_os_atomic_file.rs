use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use keyringd_core::persistence::{AtomicFileStorage, FileSaveError};

#[derive(Debug)]
pub(crate) struct MacOsAtomicFile {
    path: PathBuf,
    write_lock: std::sync::Mutex<()>,
}

impl MacOsAtomicFile {
    pub(crate) fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            write_lock: std::sync::Mutex::new(()),
        }
    }
}

impl AtomicFileStorage for MacOsAtomicFile {
    fn read(&self) -> io::Result<Option<Vec<u8>>> {
        match std::fs::read(&self.path) {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn replace(&self, bytes: &[u8]) -> Result<(), FileSaveError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| io::Error::other("file write lock was poisoned"))?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| io::Error::other("file path has no parent directory"))?;
        std::fs::create_dir_all(parent)?;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        let directory = std::fs::File::open(parent)?;
        let mut file = tempfile::Builder::new()
            .prefix(".keyringd-")
            .tempfile_in(parent)?;
        file.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
        file.write_all(bytes)?;
        file.as_file().sync_all()?;
        let file = file.persist(&self.path).map_err(|error| error.error)?;
        if unsafe { libc::fsync(directory.as_raw_fd()) } != 0 {
            return Err(FileSaveError::AfterReplace(io::Error::last_os_error()));
        }
        file.sync_all().map_err(FileSaveError::AfterReplace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_core::application::SettingStore;
    use keyringd_core::persistence::FileProfileRepository;
    use keyringd_core::persistence::{FileSettingRepository, ProfileRepository, ProfilesDocument};
    use std::sync::Arc;

    #[test]
    fn failure_injection_preserves_the_common_repository_behavior() {
        use keyringd_core::test_support::SaveFailureProfileRepository;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("profiles.json");
        let repository = FileProfileRepository::new(Arc::new(MacOsAtomicFile::new(&path)));
        let store = SaveFailureProfileRepository::new(repository);
        store.save(&ProfilesDocument::default()).unwrap();
        let saved = std::fs::read(&path).unwrap();
        store.set_save_failure(true);
        assert!(store.save(&ProfilesDocument::default()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        store.set_save_failure(false);
        std::fs::write(&path, "invalid profile document").unwrap();
        assert_eq!(
            store.load().err().unwrap().kind(),
            io::ErrorKind::InvalidData
        );
        store.save(&ProfilesDocument::default()).unwrap();
        assert!(store.load().unwrap().is_empty());
    }

    #[test]
    fn repositories_share_private_atomic_storage_without_overwriting_each_other() {
        let directory = tempfile::tempdir().unwrap();
        let profiles_path = directory.path().join("profiles.json");
        let settings_path = directory.path().join("setting.json");
        let profiles = FileProfileRepository::new(Arc::new(
            crate::system::persistence::MacOsAtomicFile::new(&profiles_path),
        ));
        let settings = SettingStore::new(Arc::new(FileSettingRepository::new(Arc::new(
            MacOsAtomicFile::new(&settings_path),
        ))))
        .unwrap();
        profiles.save(&ProfilesDocument::default()).unwrap();
        settings.set("selection", &"keyringd").unwrap();
        profiles.save(&ProfilesDocument::default()).unwrap();
        settings.set("other", &42).unwrap();
        settings.remove("other").unwrap();
        let reopened = SettingStore::new(Arc::new(FileSettingRepository::new(Arc::new(
            MacOsAtomicFile::new(&settings_path),
        ))))
        .unwrap();
        assert_eq!(
            reopened.get::<String>("selection").unwrap().as_deref(),
            Some("keyringd")
        );
        assert!(profiles.load().unwrap().is_empty());
        for path in [&profiles_path, &settings_path] {
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert_eq!(
            std::fs::metadata(directory.path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
    }

    #[test]
    fn missing_is_distinct_from_an_unreadable_path() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(
            MacOsAtomicFile::new(&directory.path().join("missing"))
                .read()
                .unwrap(),
            None
        );
        assert!(MacOsAtomicFile::new(directory.path()).read().is_err());
    }
    #[test]
    fn failed_replacement_cleans_up_the_temporary_file() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("occupied");
        std::fs::create_dir(&destination).unwrap();
        let error = MacOsAtomicFile::new(&destination)
            .replace(b"bytes")
            .unwrap_err();
        assert!(matches!(error, FileSaveError::BeforeReplace(_)));
        assert!(destination.is_dir());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
