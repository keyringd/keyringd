use std::path::{Path, PathBuf};
use std::process::Command;

use crate::core::driver::MacosTools;
use anyhow::{Result, ensure};

pub(crate) struct MacosSigningKeychain {
    path: PathBuf,
    original: Vec<String>,
    created: bool,
    search_changed: bool,
}

impl MacosSigningKeychain {
    pub(crate) fn import(
        directory: &Path,
        password: &str,
        trusted_tool: &str,
        tools: &MacosTools,
    ) -> Result<Self> {
        let original = tools
            .text(Command::new("security").args(["list-keychains", "-d", "user"]))?
            .lines()
            .map(|line| line.trim().trim_matches('"').to_owned())
            .filter(|line| !line.is_empty())
            .collect();
        let mut keychain = Self {
            path: directory.join("signing.keychain-db"),
            original,
            created: false,
            search_changed: false,
        };
        keychain.created = true; // Also clean up a partially successful create command.
        tools.checked(
            Command::new("security")
                .args(["create-keychain", "-p", password])
                .arg(&keychain.path),
        )?;
        tools.checked(
            Command::new("security")
                .args(["set-keychain-settings", "-lut", "7200"])
                .arg(&keychain.path),
        )?;
        tools.checked(
            Command::new("security")
                .args(["unlock-keychain", "-p", password])
                .arg(&keychain.path),
        )?;
        keychain.search_changed = true;
        tools.checked(
            Command::new("security")
                .args(["list-keychains", "-d", "user", "-s"])
                .arg(&keychain.path)
                .args(&keychain.original),
        )?;
        let identity = directory.join("import.p12");
        tools.checked(
            Command::new("security")
                .arg("import")
                .arg(&identity)
                .args(["-P", password, "-k"])
                .arg(&keychain.path)
                .args(["-T", trusted_tool]),
        )?;
        std::fs::remove_file(identity)?;
        tools.checked(
            Command::new("security")
                .args([
                    "set-key-partition-list",
                    "-S",
                    "apple-tool:,apple:",
                    "-s",
                    "-k",
                    password,
                ])
                .arg(&keychain.path),
        )?;
        Ok(keychain)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn cleanup(&mut self) -> Result<()> {
        // Bypass cancellation here: interrupted packaging must still restore the user's keychains.
        // Capture output so cleanup cannot accidentally disclose signing material.
        self.cleanup_with(|command| {
            ensure!(
                command.output()?.status.success(),
                "signing keychain cleanup command failed"
            );
            Ok(())
        })
    }

    fn cleanup_with(&mut self, mut run: impl FnMut(&mut Command) -> Result<()>) -> Result<()> {
        let restored = if self.search_changed {
            run(Command::new("security")
                .args(["list-keychains", "-d", "user", "-s"])
                .args(&self.original))
        } else {
            Ok(())
        };
        if restored.is_ok() {
            self.search_changed = false;
        }
        // Still attempt deletion if restoration failed, including a failure to spawn security.
        let deleted = if self.created {
            run(Command::new("security")
                .arg("delete-keychain")
                .arg(&self.path))
        } else {
            Ok(())
        };
        if deleted.is_ok() {
            self.created = false;
        }
        restored.and(deleted)
    }
}

impl Drop for MacosSigningKeychain {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            eprintln!("Signing keychain cleanup failed: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "MacosSigningKeychain::cleanup_with" {
            case "Cleanup restores even an empty search list then deletes the keychain" {
                verify "restore the original keychain search list before deleting the temporary keychain" {
                    let mut keychain = MacosSigningKeychain {
                        path: "temporary signing.keychain-db".into(),
                        original: vec![],
                        created: true,
                        search_changed: true,
                    };
                    let mut calls = Vec::new();
                    keychain
                        .cleanup_with(|command| {
                            calls.push(
                                command
                                    .get_args()
                                    .map(|arg| arg.to_string_lossy().into_owned())
                                    .collect::<Vec<_>>(),
                            );
                            Ok(())
                        })
                        .unwrap();
                    assert_eq!(
                        calls,
                        [
                            vec!["list-keychains", "-d", "user", "-s"],
                            vec!["delete-keychain", "temporary signing.keychain-db"]
                        ]
                    );
                    assert!(!keychain.created && !keychain.search_changed);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "MacosSigningKeychain::cleanup_with" {
            case "Failed restoration still deletes and can be retried" {
                verify "delete the temporary keychain after a restore error and allow cleanup to retry" {
                    let mut keychain = MacosSigningKeychain {
                        path: "temporary signing.keychain-db".into(),
                        original: vec!["original keychain".into()],
                        created: true,
                        search_changed: true,
                    };
                    let mut calls = 0;
                    let result = keychain.cleanup_with(|_| {
                        calls += 1;
                        ensure!(calls != 1, "restore failed");
                        Ok(())
                    });
                    assert!(result.is_err());
                    assert_eq!(calls, 2);
                    assert!(!keychain.created && keychain.search_changed);
                    keychain
                        .cleanup_with(|command| {
                            assert_eq!(command.get_args().last().unwrap(), "original keychain");
                            Ok(())
                        })
                        .unwrap();
                }
            }
        }
    }
}
