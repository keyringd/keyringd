use anyhow::{Context, Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// grcov aggregates the normalized traces and generates both portable data and HTML.
pub(in crate::core) fn render(root: &Path, inputs: &[PathBuf], output: &Path) -> Result<()> {
    fs::create_dir_all(output)?;
    let output = output.canonicalize()?;
    let status = Command::new("grcov")
        .current_dir(root)
        .args(inputs)
        .arg("--source-dir")
        .arg(root)
        .arg("--no-demangle")
        .arg("--output-types")
        .arg("html,lcov")
        .arg("--output-path")
        .arg(output)
        .status()
        .context("starting grcov; install grcov to aggregate and render coverage")?;
    ensure!(
        status.success(),
        "generating grcov coverage report failed: {status}"
    );
    Ok(())
}
