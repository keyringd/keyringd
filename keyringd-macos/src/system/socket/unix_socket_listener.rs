use std::fs::File;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use keyringd_ssh_agent_protocol::SshAgentListener;
use tokio::net::{UnixListener, UnixStream};

#[derive(Debug)]
pub(crate) struct UnixSocketListener {
    listener: UnixListener,
    path: PathBuf,
    // Held for the listener's entire lifetime, including startup and cleanup.
    _lock: File,
}

impl UnixSocketListener {
    pub(crate) fn bind(path: &Path) -> std::io::Result<Self> {
        let parent = path
            .parent()
            .ok_or_else(|| std::io::Error::other("Missing agent directory."))?;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
        let metadata = std::fs::symlink_metadata(parent)?;
        if !metadata.is_dir() || metadata.uid() != unsafe { libc::getuid() } {
            return Err(std::io::Error::other(
                "The agent directory must be owned by this user.",
            ));
        }
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(parent.join("agent.lock"))?;
        lock.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                "Keyringd is already running.",
            ),
            std::fs::TryLockError::Error(error) => error,
        })?;
        Self::remove_stale_socket(path)?;
        let listener = UnixListener::bind(path)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        Ok(Self {
            listener,
            path: path.into(),
            _lock: lock,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    fn remove_stale_socket(path: &Path) -> std::io::Result<()> {
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        if !metadata.file_type().is_socket() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("refusing to replace non-socket path {}", path.display()),
            ));
        }
        if std::os::unix::net::UnixStream::connect(path).is_ok() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                format!("Unix socket is already in use at {}", path.display()),
            ));
        }
        std::fs::remove_file(path)
    }
}

impl Drop for UnixSocketListener {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                tracing::warn!(%error, path = %self.path.display(), "failed to remove Unix socket")
            }
        }
    }
}

impl SshAgentListener for UnixSocketListener {
    type Stream = UnixStream;
    async fn accept(&mut self) -> std::io::Result<Self::Stream> {
        self.listener.accept().await.map(|(stream, _)| stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    keyringd_testkit::scenario! {
        group "UnixSocketListener::bind" {
            case "Rejecting Sockets owned by Server in operation" {
                given "Prepare a UnixListener" {
                    let directory = tempfile::tempdir_in("/tmp").unwrap();
                    let path = directory.path().join("agent.sock");
                    let _active = std::os::unix::net::UnixListener::bind(&path).unwrap();
                }
                when "Call UnixSocketListener::bind" {
                    let error = UnixSocketListener::bind(&path).unwrap_err();
                }
                then "Verify that the results match ErrorKind::AddrInUse" {
                    assert_eq!(error.kind(), std::io::ErrorKind::AddrInUse);
                    assert!(path.exists());
                }
            }
            case "Rejects Normal Files in Socket Path" {
                given "Prepare Directory Path" {
                    let directory = tempfile::tempdir_in("/tmp").unwrap();
                    let path = directory.path().join("agent.sock");
                    std::fs::write(&path, b"keep").unwrap();
                }
                when "Call UnixSocketListener::bind" {
                    let error = UnixSocketListener::bind(&path).unwrap_err();
                }
                then "Verify that the results match ErrorKind::InvalidInput" {
                    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
                    assert_eq!(std::fs::read(&path).unwrap(), b"keep");
                }
            }
            case "Reject Symbolic Link in Socket Path" {
                given "Prepare Directory, Target, and Path" {
                    let directory = tempfile::tempdir_in("/tmp").unwrap();
                    let target = directory.path().join("target");
                    let path = directory.path().join("agent.sock");
                    std::fs::write(&target, b"keep").unwrap();
                    symlink(&target, &path).unwrap();
                }
                when "Call UnixSocketListener::bind" {
                    let error = UnixSocketListener::bind(&path).unwrap_err();
                }
                then "Verify that the results match ErrorKind::InvalidInput" {
                    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
                    assert_eq!(std::fs::read(&target).unwrap(), b"keep");
                }
            }
            case "Replace unconnectable old Sockets" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                given "Prepare a UnixListener" {
                    let directory = tempfile::tempdir_in("/tmp").unwrap();
                    let path = directory.path().join("agent.sock");
                    drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
                }
                when "Call UnixSocketListener::bind" {
                    let listener = UnixSocketListener::bind(&path).unwrap();
                }
                then "Replace unconnectable old Sockets" {
                    assert!(path.exists());
                    drop(listener);
                    assert!(!path.exists());
                }
            }
        }
    }
}
