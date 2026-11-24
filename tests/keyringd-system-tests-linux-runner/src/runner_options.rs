use std::path::PathBuf;

use anyhow::{Result, ensure};

use crate::Distribution;

pub struct RunnerOptions {
    pub workspace_root: PathBuf,
    pub report_directory: PathBuf,
    pub distribution: Distribution,
    pub package: Option<PathBuf>,
    pub filter: Option<String>,
}

impl RunnerOptions {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            self.workspace_root.is_absolute(),
            "workspace root must be absolute"
        );
        ensure!(
            self.workspace_root.join("Cargo.toml").is_file(),
            "workspace Cargo.toml is missing"
        );
        Ok(())
    }
}
