use std::os::fd::FromRawFd;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};

use tokio::net::{UnixListener, UnixStream};

const SYSTEMD_LISTEN_FDS_START: i32 = 3;

#[derive(Debug)]
pub(crate) struct UnixSocketListener {
    listener: UnixListener,
    owned_path: Option<PathBuf>,
}

impl UnixSocketListener {
    pub(crate) fn bind(path: &Path) -> std::io::Result<Self> {
        Self::remove_stale_socket(path)?;
        let listener = UnixListener::bind(path)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        Ok(Self {
            listener,
            owned_path: Some(path.to_path_buf()),
        })
    }

    pub(crate) fn from_systemd() -> std::io::Result<Option<Self>> {
        let Some(listen_pid) = std::env::var_os("LISTEN_PID") else {
            return Ok(None);
        };
        let listen_pid = listen_pid
            .to_string_lossy()
            .parse::<u32>()
            .map_err(|_| Self::invalid_input("LISTEN_PID is not a valid process ID"))?;
        if listen_pid != std::process::id() {
            return Ok(None);
        }
        let listen_fds = std::env::var("LISTEN_FDS")
            .map_err(|_| Self::invalid_input("LISTEN_PID is set but LISTEN_FDS is missing"))?
            .parse::<u32>()
            .map_err(|_| Self::invalid_input("LISTEN_FDS is not a valid descriptor count"))?;
        if listen_fds != 1 {
            return Err(Self::invalid_input(format!(
                "expected one systemd socket, received {listen_fds}"
            )));
        }

        let standard =
            unsafe { std::os::unix::net::UnixListener::from_raw_fd(SYSTEMD_LISTEN_FDS_START) };
        standard.set_nonblocking(true)?;
        let flags = unsafe { libc::fcntl(SYSTEMD_LISTEN_FDS_START, libc::F_GETFD) };
        if flags == -1
            || unsafe {
                libc::fcntl(
                    SYSTEMD_LISTEN_FDS_START,
                    libc::F_SETFD,
                    flags | libc::FD_CLOEXEC,
                )
            } == -1
        {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Some(Self {
            listener: UnixListener::from_std(standard)?,
            owned_path: None,
        }))
    }

    pub(crate) async fn accept(&self) -> std::io::Result<UnixStream> {
        self.listener.accept().await.map(|(stream, _)| stream)
    }

    fn remove_stale_socket(path: &Path) -> std::io::Result<()> {
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        if !metadata.file_type().is_socket() {
            return Err(Self::invalid_input(format!(
                "refusing to replace non-socket path {}",
                path.display()
            )));
        }
        if std::os::unix::net::UnixStream::connect(path).is_ok() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                format!("Unix socket is already in use at {}", path.display()),
            ));
        }
        std::fs::remove_file(path)
    }

    fn invalid_input(message: impl Into<String>) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, message.into())
    }
}

impl Drop for UnixSocketListener {
    fn drop(&mut self) {
        let Some(path) = &self.owned_path else {
            return;
        };
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "failed to remove Unix socket")
            }
        }
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
                    let directory = tempfile::tempdir().unwrap();
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
                    let directory = tempfile::tempdir().unwrap();
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
                    let directory = tempfile::tempdir().unwrap();
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
                    let directory = tempfile::tempdir().unwrap();
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
