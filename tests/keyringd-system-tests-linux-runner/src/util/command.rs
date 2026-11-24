use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use anyhow::{Context, Result, ensure};

pub(crate) fn run(
    program: impl AsRef<OsStr>,
    arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
    directory: &Path,
) -> Result<ExitStatus> {
    let program = program.as_ref();
    let status = Command::new(program)
        .args(arguments)
        .current_dir(directory)
        .status()
        .with_context(|| format!("failed to execute {}", program.to_string_lossy()))?;
    Ok(status)
}

pub(crate) fn require_success(
    description: &str,
    program: impl AsRef<OsStr>,
    arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
    directory: &Path,
) -> Result<()> {
    let status = run(program, arguments, directory)?;
    ensure!(status.success(), "{description} failed with {status}");
    Ok(())
}

pub(crate) fn output(
    program: impl AsRef<OsStr>,
    arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
    directory: &Path,
) -> Result<String> {
    let program = program.as_ref();
    let output = Command::new(program)
        .args(arguments)
        .current_dir(directory)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("failed to execute {}", program.to_string_lossy()))?;
    ensure!(
        output.status.success(),
        "{} failed: {}",
        program.to_string_lossy(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).context("command output is not UTF-8")
}
