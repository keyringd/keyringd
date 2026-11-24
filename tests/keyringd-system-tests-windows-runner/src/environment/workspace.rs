use crate::RunnerOptions;
use anyhow::{Context, Result};
use std::path::PathBuf;

pub(crate) struct Workspace {
    pub root: PathBuf,
    pub cache: PathBuf,
}
impl Workspace {
    pub fn new(options: &RunnerOptions) -> Result<Self> {
        let cache = PathBuf::from(
            std::env::var_os("LOCALAPPDATA")
                .context("LOCALAPPDATA is required for Windows UI builds")?,
        )
        .join("keyringd-ui-tests");
        std::fs::create_dir_all(&cache)?;
        Ok(Self {
            root: options.workspace_root.clone(),
            cache,
        })
    }
}
