use std::fs::{File, OpenOptions};
use std::path::Path;

use anyhow::{Context, Result, bail};

/// Keeps the host's fixed USB/IP and Hybrid tunnel resources exclusive to one E2E run,
/// including runs started from another checkout.
pub(crate) struct RunLock {
    _file: File,
}

impl RunLock {
    pub(crate) fn acquire() -> Result<Self> {
        Self::acquire_at(&std::env::temp_dir().join("keyringd-linux-e2e.lock"))
    }

    fn acquire_at(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| format!("open E2E run lock {}", path.display()))?;
        if let Err(error) = file.try_lock() {
            match error {
                std::fs::TryLockError::WouldBlock => {
                    bail!(
                        "another Linux system test run is already active on this host ({})",
                        path.display()
                    );
                }
                std::fs::TryLockError::Error(error) => {
                    return Err(error).with_context(|| format!("lock E2E run {}", path.display()));
                }
            }
        }
        Ok(Self { _file: file })
    }
}

#[cfg(test)]
mod tests {
    use super::RunLock;

    keyringd_testkit::scenario! {
        group "RunLock::acquire_at" {
            case "Rejects a second run until the first finishes" {
                verify "refuse a second test run while the first run holds the lock" {
                    let workspace = tempfile::tempdir().unwrap();
                    let path = workspace.path().join("run.lock");
                    let first = RunLock::acquire_at(&path)
                        .expect("the first run should acquire the lock because no other run owns it");
                    assert!(RunLock::acquire_at(&path).is_err());
                    drop(first);
                    RunLock::acquire_at(&path).expect(
                        "the next run should acquire the lock because dropping the previous run releases it",
                    );
                }
            }
        }
    }
}
