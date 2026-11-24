use std::fs::File;
use std::io;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, symlink};
use std::path::{Path, PathBuf};

/// Swaps a session's agent socket without
/// changing the environment inherited by already running SSH clients.
pub(super) struct SshAgentSocket {
    directory: PathBuf,
    socket: PathBuf,
}

impl SshAgentSocket {
    pub(super) fn new(directory: PathBuf, socket: PathBuf) -> Self {
        Self { directory, socket }
    }

    fn backup_path(source: &Path) -> PathBuf {
        let mut name = source.as_os_str().to_owned();
        name.push(".keyringd-original");
        name.into()
    }

    pub(super) fn configure(&self, source: &Path) -> io::Result<()> {
        let _lock = self.lock()?;
        self.validate_source(source)?;
        Self::require_socket(&self.socket)?;

        self.select(source)
    }

    pub(super) fn unconfigure(&self, source: &Path) -> io::Result<()> {
        let _lock = self.lock()?;
        self.validate_source(source)?;

        self.restore(source)
    }

    fn lock(&self) -> io::Result<File> {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.directory)?;
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.directory.join("ssh-agent-socket.lock"))?;
        lock.lock()?;
        Ok(lock)
    }

    fn validate_source(&self, source: &Path) -> io::Result<()> {
        if !source.is_absolute() || source == self.socket {
            return Err(io::Error::other("Invalid default SSH agent socket path."));
        }
        let parent = source
            .parent()
            .ok_or_else(|| io::Error::other("Missing socket directory."))?;
        let metadata = std::fs::metadata(parent)?;
        if metadata.uid() != unsafe { libc::getuid() } || metadata.mode() & 0o022 != 0 {
            return Err(io::Error::other(
                "The default SSH agent directory is not private to this user.",
            ));
        }
        Ok(())
    }

    fn require_socket(path: &Path) -> io::Result<()> {
        let metadata = std::fs::symlink_metadata(path)?;
        if !metadata.file_type().is_socket() || metadata.uid() != unsafe { libc::getuid() } {
            return Err(io::Error::other(format!(
                "Expected an owned Unix socket at {}.",
                path.display()
            )));
        }
        Ok(())
    }

    pub(super) fn is_own_link(&self, source: &Path) -> io::Result<bool> {
        match std::fs::read_link(source) {
            Ok(target) => Ok(target == self.socket),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::InvalidInput
                ) =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    fn select(&self, source: &Path) -> io::Result<()> {
        let backup = Self::backup_path(source);
        if self.is_own_link(source)? {
            return Self::require_socket(&backup);
        }
        match std::fs::symlink_metadata(&backup) {
            Ok(_) => {
                Self::require_socket(&backup)?;
                // Recover a process that exited between rename and symlink.
                match std::fs::symlink_metadata(source) {
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                    Ok(_) => {
                        return Err(io::Error::other(
                            "The SSH agent backup already exists; leaving both files untouched.",
                        ));
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Self::require_socket(source)?;
                std::fs::rename(source, &backup)?;
            }
            Err(error) => return Err(error),
        }
        if let Err(error) = symlink(&self.socket, source) {
            // Do not replace a file created by someone else in the meantime.
            if std::fs::symlink_metadata(source)
                .is_err_and(|error| error.kind() == io::ErrorKind::NotFound)
            {
                std::fs::rename(&backup, source)?;
            }
            return Err(error);
        }
        Ok(())
    }

    fn restore(&self, source: &Path) -> io::Result<()> {
        let backup = Self::backup_path(source);
        match std::fs::symlink_metadata(&backup) {
            Ok(_) => Self::require_socket(&backup)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if self.is_own_link(source)? {
                    return Err(io::Error::other(
                        "The original SSH agent socket is missing; leaving the link untouched.",
                    ));
                }
                return Ok(());
            }
            Err(error) => return Err(error),
        }
        if !self.is_own_link(source)? {
            match std::fs::symlink_metadata(source) {
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
                Ok(_) => {
                    return Err(io::Error::other(
                        "The default SSH agent was changed externally; leaving it untouched.",
                    ));
                }
            }
        }
        // rename replaces our link atomically, preserving the listening socket.
        std::fs::rename(backup, source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::{UnixListener, UnixStream};

    #[test]
    fn restores_the_original_listener_without_restarting_clients() {
        let directory = tempfile::tempdir_in("/tmp").unwrap();
        let source = directory.path().join("Listeners");
        let target = directory.path().join("agent.sock");
        let original = UnixListener::bind(&source).unwrap();
        let keyringd = UnixListener::bind(&target).unwrap();
        let selection = SshAgentSocket::new(directory.path().join("state"), target.clone());
        selection.configure(&source).unwrap();
        let client = UnixStream::connect(&source).unwrap();
        let _ = keyringd.accept().unwrap();
        drop(client);
        // A new instance models restarting the app in the same login session.
        let selection = SshAgentSocket::new(directory.path().join("state"), target);
        selection.configure(&source).unwrap();
        selection.unconfigure(&source).unwrap();
        let _client = UnixStream::connect(&source).unwrap();
        let _ = original.accept().unwrap();
        assert!(!selection.is_own_link(&source).unwrap());
        assert!(!SshAgentSocket::backup_path(&source).exists());
    }

    #[test]
    fn resumes_after_rename_for_either_operation() {
        for enabled in [true, false] {
            let directory = tempfile::tempdir_in("/tmp").unwrap();
            let source = directory.path().join("Listeners");
            let target = directory.path().join("agent.sock");
            let _original = UnixListener::bind(&source).unwrap();
            let _keyringd = UnixListener::bind(&target).unwrap();
            let selection = SshAgentSocket::new(directory.path().join("state"), target.clone());
            let lock = selection.lock().unwrap();

            std::fs::rename(&source, SshAgentSocket::backup_path(&source)).unwrap();
            drop(lock);
            if enabled {
                selection.configure(&source).unwrap();
            } else {
                selection.unconfigure(&source).unwrap();
            }
            assert_eq!(std::fs::read_link(&source).ok(), enabled.then_some(target));
            assert_eq!(SshAgentSocket::backup_path(&source).exists(), enabled);
        }
    }

    #[test]
    fn preserves_foreign_files_and_a_missing_backup() {
        let directory = tempfile::tempdir_in("/tmp").unwrap();
        let source = directory.path().join("Listeners");
        let target = directory.path().join("agent.sock");
        let _original = UnixListener::bind(&source).unwrap();
        let _keyringd = UnixListener::bind(&target).unwrap();
        let selection = SshAgentSocket::new(directory.path().join("state"), target.clone());
        let backup = SshAgentSocket::backup_path(&source);
        std::fs::write(&backup, b"keep").unwrap();
        assert!(selection.configure(&source).is_err());
        assert_eq!(std::fs::read(&backup).unwrap(), b"keep");
        std::fs::remove_file(&backup).unwrap();
        selection.configure(&source).unwrap();
        std::fs::remove_file(&source).unwrap();
        std::fs::write(&source, b"external change").unwrap();
        assert!(selection.unconfigure(&source).is_err());
        assert_eq!(std::fs::read(&source).unwrap(), b"external change");
        std::fs::remove_file(&source).unwrap();
        symlink(&target, &source).unwrap();
        std::fs::remove_file(&backup).unwrap();
        assert!(selection.unconfigure(&source).is_err());
        assert_eq!(std::fs::read_link(&source).unwrap(), target);
    }

    #[test]
    fn can_select_and_restore_different_login_sockets() {
        let directory = tempfile::tempdir_in("/tmp").unwrap();
        let target = directory.path().join("agent.sock");
        let _keyringd = UnixListener::bind(&target).unwrap();
        let selection = SshAgentSocket::new(directory.path().join("state"), target.clone());
        let first = directory.path().join("first-login");
        let _first = UnixListener::bind(&first).unwrap();
        selection.configure(&first).unwrap();
        let second = directory.path().join("second-login");
        let original = UnixListener::bind(&second).unwrap();
        selection.configure(&second).unwrap();
        assert_eq!(std::fs::read_link(&second).unwrap(), target);
        selection.unconfigure(&second).unwrap();
        let _client = UnixStream::connect(&second).unwrap();
        let _ = original.accept().unwrap();
        let third = directory.path().join("third-login");
        let _third = UnixListener::bind(&third).unwrap();
        selection.unconfigure(&third).unwrap();
        assert!(
            std::fs::symlink_metadata(third)
                .unwrap()
                .file_type()
                .is_socket()
        );
    }

    #[test]
    fn a_link_to_a_stopped_agent_fails_instead_of_waiting_for_activation() {
        let directory = tempfile::tempdir_in("/tmp").unwrap();
        let source = directory.path().join("Listeners");
        let target = directory.path().join("agent.sock");
        let _original = UnixListener::bind(&source).unwrap();
        let keyringd = UnixListener::bind(&target).unwrap();
        let selection = SshAgentSocket::new(directory.path().join("state"), target);
        selection.configure(&source).unwrap();
        drop(keyringd);
        assert_eq!(
            UnixStream::connect(&source).unwrap_err().kind(),
            io::ErrorKind::ConnectionRefused
        );
        selection.unconfigure(&source).unwrap();
        assert!(UnixStream::connect(source).is_ok());
    }
}
