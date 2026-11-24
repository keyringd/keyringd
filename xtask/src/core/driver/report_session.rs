use crate::core::model::Workspace;
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
pub(crate) struct ReportSession<'a> {
    workspace: &'a Workspace,
    directory: PathBuf,
}
impl<'a> ReportSession<'a> {
    pub(crate) fn start(workspace: &'a Workspace, directory: PathBuf) -> Result<Self> {
        let directory = if directory.is_absolute() {
            directory
        } else {
            workspace.root().join(directory)
        };
        ensure!(
            !directory
                .components()
                .any(|part| part == Component::ParentDir),
            "report directory must not contain '..'"
        );
        let target = Self::resolve(&workspace.target())?;
        let resolved = Self::resolve(&directory)?;
        ensure!(
            resolved.starts_with(&target) && resolved != target,
            "report directory must be a subdirectory of {}",
            workspace.target().display()
        );
        let report = Self {
            workspace,
            directory,
        };
        report.reset()?;
        Ok(report)
    }
    // Resolve existing ancestors as well, so a symlink cannot redirect cleanup
    // outside target even when the final report directory does not exist yet.
    fn resolve(path: &Path) -> Result<PathBuf> {
        let mut ancestor = path;
        let mut missing = Vec::new();
        while !ancestor.try_exists()? {
            missing.push(ancestor.file_name().context("invalid report directory")?);
            ancestor = ancestor
                .parent()
                .context("report directory has no parent")?;
        }
        let mut resolved = ancestor.canonicalize()?;
        resolved.extend(missing.into_iter().rev());
        Ok(resolved)
    }
    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }
    fn reset(&self) -> Result<()> {
        if self.directory.exists() {
            fs::remove_dir_all(&self.directory).with_context(|| {
                format!(
                    "failed to reset test report directory {}",
                    self.directory.display()
                )
            })?;
        }
        fs::create_dir_all(&self.directory).with_context(|| {
            format!(
                "failed to create test report directory {}",
                self.directory.display()
            )
        })?;
        keyringd_testkit::runner::ReportRunMetadata::capture(self.workspace.root())
            .write(&self.directory)?;
        Ok(())
    }
    pub(crate) fn render(&self, include_requirements: bool) -> Result<()> {
        keyringd_testkit::generate_html_report_with_options(
            &self.directory,
            &keyringd_testkit::ReportOptions {
                include_requirements,
                ..Default::default()
            },
        )
        .context("failed to generate the HTML test report")?;
        println!(
            "Test report: {}",
            self.directory.join("index.html").display()
        );
        Ok(())
    }
}
