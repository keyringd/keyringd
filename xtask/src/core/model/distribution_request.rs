use super::{DistFormat, DistTarget};
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
#[derive(Clone, Debug)]
pub(crate) struct DistributionRequest {
    pub(crate) format: DistFormat,
    pub(crate) ad_hoc: bool,
    pub(crate) target: DistTarget,
    pub(crate) output_dir: PathBuf,
    pub(crate) binary: PathBuf,
    pub(crate) windows_exe: Option<PathBuf>,
    pub(crate) windows_runtime_dir: Option<PathBuf>,
}
impl DistributionRequest {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            matches!(
                (&self.target, &self.format),
                (
                    DistTarget::Linux | DistTarget::Wsl,
                    DistFormat::Deb | DistFormat::Rpm
                ) | (DistTarget::Windows, DistFormat::Msi | DistFormat::Runtime)
                    | (DistTarget::Macos, DistFormat::App)
            ),
            "unsupported package target and format"
        );
        ensure!(
            !self.ad_hoc
                || matches!(
                    (&self.target, &self.format),
                    (DistTarget::Macos, DistFormat::App)
                ),
            "--ad-hoc requires --target macos --format app"
        );
        if matches!(self.target, DistTarget::Wsl) {
            self.windows_inputs()?;
        }
        Ok(())
    }

    pub(crate) fn windows_inputs(&self) -> Result<(&Path, &Path)> {
        let executable = self
            .windows_exe
            .as_deref()
            .context("--windows-exe is required for wsl target")?;
        let runtime_directory = self
            .windows_runtime_dir
            .as_deref()
            .context("--windows-runtime-dir is required for wsl target")?;
        Ok((executable, runtime_directory))
    }
}
