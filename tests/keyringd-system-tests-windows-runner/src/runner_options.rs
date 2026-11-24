#[cfg(windows)]
use anyhow::{Result, ensure};
use std::path::PathBuf;
/// Execution configuration, independent of CLI syntax.
#[derive(Clone, Debug)]
pub struct RunnerOptions {
    pub suite: crate::TestSuite,
    pub workspace_root: PathBuf,
    pub report_directory: PathBuf,
    pub filter: Option<String>,
}
impl RunnerOptions {
    #[cfg(windows)]
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            self.workspace_root.is_absolute() && self.workspace_root.join("Cargo.toml").is_file(),
            "workspace root must be an absolute path containing Cargo.toml"
        );
        ensure!(
            self.report_directory.is_absolute(),
            "report directory must be absolute"
        );
        Ok(())
    }
}
