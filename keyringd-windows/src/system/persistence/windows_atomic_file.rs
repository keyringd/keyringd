use std::io::{self, Write};
use std::path::{Path, PathBuf};

use keyringd_core::persistence::{AtomicFileStorage, FileSaveError};

#[derive(Debug)]
pub(crate) struct WindowsAtomicFile {
    path: PathBuf,
    write_lock: std::sync::Mutex<()>,
}

impl WindowsAtomicFile {
    pub(crate) fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            write_lock: std::sync::Mutex::new(()),
        }
    }
}

impl AtomicFileStorage for WindowsAtomicFile {
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
        let mut file = tempfile::Builder::new()
            .prefix(".keyringd-")
            .tempfile_in(parent)?;
        file.write_all(bytes)?;
        file.as_file().sync_all()?;
        let temporary = file.into_temp_path();
        replace_file(&temporary, &self.path)?;
        Ok(())
    }
}

fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    use windows::core::PCWSTR;
    let source: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(target.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(io::Error::other)
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
    fn repositories_share_atomic_storage_without_overwriting_each_other() {
        let directory = tempfile::tempdir().unwrap();
        let profiles = FileProfileRepository::new(Arc::new(
            crate::system::persistence::WindowsAtomicFile::new(
                &directory.path().join("profiles.json"),
            ),
        ));
        let path = directory.path().join("setting.json");
        let store = SettingStore::new(Arc::new(FileSettingRepository::new(Arc::new(
            WindowsAtomicFile::new(&path),
        ))))
        .unwrap();
        profiles.save(&ProfilesDocument::default()).unwrap();
        store.set("selected", &true).unwrap();
        profiles.save(&ProfilesDocument::default()).unwrap();
        let reopened = SettingStore::new(Arc::new(FileSettingRepository::new(Arc::new(
            WindowsAtomicFile::new(&path),
        ))))
        .unwrap();
        assert_eq!(reopened.get::<bool>("selected").unwrap(), Some(true));
        assert!(profiles.load().unwrap().is_empty());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
    }
    #[test]
    fn failed_replacement_cleans_up_the_temporary_file() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("occupied");
        std::fs::create_dir(&destination).unwrap();
        let error = WindowsAtomicFile::new(&destination)
            .replace(b"bytes")
            .unwrap_err();
        assert!(matches!(error, FileSaveError::BeforeReplace(_)));
        assert!(destination.is_dir());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
