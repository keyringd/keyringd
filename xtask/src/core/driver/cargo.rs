use crate::core::model::Workspace;
use anyhow::{Context, Result, ensure};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn build(
    workspace: &Workspace,
    packages: &[&str],
    release: bool,
    locked: bool,
) -> Result<()> {
    let mut command = Command::new(executable());
    command.current_dir(workspace.root()).arg("build");
    for package in packages {
        command.args(["--package", package]);
    }
    if release {
        command.arg("--release");
    }
    if locked {
        command.arg("--locked");
    }
    let status = command.status().context("could not start cargo build")?;
    ensure!(status.success(), "cargo build failed with {status}");
    Ok(())
}
pub(crate) fn test(workspace: &Workspace, arguments: &[OsString], report: &Path) -> Result<()> {
    let status = Command::new(executable())
        .arg("test")
        .args(arguments)
        .current_dir(workspace.root())
        .env("KEYRINGD_TEST_REPORT_DIR", report)
        .status()
        .context("failed to execute cargo test")?;
    ensure!(status.success(), "cargo test failed with {status}");
    Ok(())
}

pub(crate) fn refresh_lockfile(root: &Path) -> Result<()> {
    super::process::run_command(
        Command::new(executable())
            // Resolving dependencies also updates workspace package versions in
            // Cargo.lock; --no-deps only reads manifests and leaves it stale.
            .args(["metadata", "--format-version", "1"])
            .stdout(std::process::Stdio::null())
            .current_dir(root),
    )
}

fn executable() -> PathBuf {
    std::env::var_os("CARGO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cargo"))
}
