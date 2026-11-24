#![cfg_attr(not(target_os = "macos"), allow(dead_code))]
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};
pub(crate) struct Workspace {
    root: PathBuf,
}
impl Workspace {
    pub(crate) fn new(root: PathBuf) -> Result<Self> {
        ensure!(
            root.is_absolute() && root.join("Cargo.toml").is_file(),
            "workspace root must be an absolute path containing Cargo.toml"
        );
        Ok(Self { root })
    }
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
    pub(crate) fn cargo(&self) -> PathBuf {
        std::env::var_os("CARGO")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("cargo"))
    }
}
