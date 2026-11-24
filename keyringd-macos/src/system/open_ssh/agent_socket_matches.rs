use std::path::Path;

pub(super) fn agent_socket_matches(agent: &Path, socket: &Path) -> std::io::Result<bool> {
    let agent = agent.canonicalize()?;
    let socket = socket.canonicalize()?;
    Ok(agent == socket)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn directory() -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "keyringd-agent-path-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }

    keyringd_testkit::scenario! {
        group "agent_socket_matches" {
            case "Resolves different spellings of the same path" {
                verify "report an identical socket for equivalent path spellings" {
                    let directory = directory();
                    std::fs::create_dir(directory.join("child")).unwrap();
                    let socket = directory.join("agent.sock");
                    std::fs::write(&socket, []).unwrap();
                    let alias = directory.join("child/../agent.sock");
                    assert!(agent_socket_matches(&alias, &socket).unwrap());
                    std::fs::remove_dir_all(directory).unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "agent_socket_matches" {
            case "Preserves the os error when identical paths cannot be resolved" {
                verify "return the filesystem error when identical socket paths cannot be resolved" {
                    let directory = directory();
                    let socket = directory.join("missing.sock");
                    assert!(!socket.exists());
                    let error = agent_socket_matches(&socket, &socket).unwrap_err();
                    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
                    assert!(error.raw_os_error().is_some());
                    std::fs::remove_dir_all(directory).unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "agent_socket_matches" {
            case "Distinguishes other paths from resolution errors" {
                verify "distinguish different sockets from a failed filesystem lookup" {
                    let directory = directory();
                    let socket = directory.join("agent.sock");
                    let other = directory.join("other.sock");
                    std::fs::write(&socket, []).unwrap();
                    std::fs::write(&other, []).unwrap();
                    assert!(!agent_socket_matches(&other, &socket).unwrap());
                    let missing = directory.join("missing.sock");
                    assert_eq!(
                        agent_socket_matches(&missing, &socket).unwrap_err().kind(),
                        std::io::ErrorKind::NotFound
                    );
                    assert_eq!(
                        agent_socket_matches(&socket, &missing).unwrap_err().kind(),
                        std::io::ErrorKind::NotFound
                    );
                    std::fs::remove_dir_all(directory).unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "agent_socket_matches" {
            case "Accepts a symlinked directory for the same unix socket" {
                verify "recognize the same socket through a symlink to its directory" {
                    // macOS's default temporary directory can exceed the Unix socket path limit.
                    let directory = tempfile::Builder::new()
                        .prefix("p4s-")
                        .tempdir_in("/tmp")
                        .unwrap();
                    let real = directory.path().join("real");
                    let alias = directory.path().join("alias");
                    std::fs::create_dir(&real).unwrap();
                    std::os::unix::fs::symlink(&real, &alias).unwrap();
                    let socket = real.join("agent.sock");
                    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
                    assert!(agent_socket_matches(&alias.join("agent.sock"), &socket).unwrap());
                    drop(listener);
                }
            }
        }
    }
}
