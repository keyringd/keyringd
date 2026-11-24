use crate::core::{
    driver::{CoverageSession, coverage_rendering},
    model::{CoverageReport, Workspace},
    service::workspace,
};
use anyhow::{Context, Result, ensure};
use std::{ffi::OsString, fs, path::Path, process::Command};

pub(crate) fn compiler_command() -> Result<Option<Command>> {
    crate::core::driver::coverage_compiler::command(
        &std::env::args_os().skip(1).collect::<Vec<_>>(),
        std::env::var_os("KEYRINGD_COVERAGE_ACTIVE").is_some(),
    )
}

pub(crate) fn run(suite: &str, os: &str) -> Result<()> {
    ensure!(
        std::env::var_os("KEYRINGD_COVERAGE_ACTIVE").is_none(),
        "nested coverage sessions are not supported"
    );
    let workspace = workspace::load(None)?;
    let session = CoverageSession::new(&workspace, &format!("{suite}-{os}"))?;
    // Preserve flags passed through to cargo test, even if a test filter happens
    // to be named --coverage. Only remove xtask's flag before the `--` boundary.
    let mut cargo_arguments = false;
    let arguments = std::env::args_os().skip(1).filter(|arg| {
        if arg == "--" {
            cargo_arguments = true;
        }
        cargo_arguments || arg != "--coverage"
    });
    let tests = session.run(arguments);
    let coverage = session.export(&revision(workspace.root())?, suite, os);
    keyringd_testkit::runner::StageFailures::finish([("tests", tests), ("coverage", coverage)])
}

pub(crate) fn build(workspace: &Workspace, arguments: &[OsString]) -> Result<()> {
    CoverageSession::new(workspace, "packaged")?.build(arguments)
}

pub(crate) fn revision(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()?;
    ensure!(
        output.status.success(),
        "cannot determine coverage source revision"
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

pub(crate) fn merge(
    workspace: &Workspace,
    input: &Path,
    output: &Path,
    required: &[String],
) -> Result<()> {
    let revision = revision(workspace.root())?;
    let mut artifacts = Vec::new();
    find_artifacts(input, &mut artifacts)?;
    ensure!(
        !artifacts.is_empty(),
        "no coverage artifacts found in {}",
        input.display()
    );
    let mut combined = Vec::new();
    let mut environments = std::collections::BTreeSet::new();
    for path in artifacts {
        let (_, suite, os) = CoverageReport::read_artifact(workspace.root(), &path, &revision)
            .with_context(|| format!("loading coverage artifact {}", path.display()))?;
        combined.push(path.join("lcov.info").canonicalize()?);
        environments.insert(format!("{suite}/{os}"));
    }
    for name in required {
        ensure!(
            environments.contains(name),
            "required coverage result is missing: {name}"
        );
    }
    coverage_rendering::render(workspace.root(), &combined, output)?;
    println!(
        "Combined coverage: {}",
        output.join("html/index.html").display()
    );
    Ok(())
}

fn find_artifacts(directory: &Path, output: &mut Vec<std::path::PathBuf>) -> Result<()> {
    if directory.join("metadata.json").is_file() && directory.join("lcov.info").is_file() {
        output.push(directory.to_owned());
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            find_artifacts(&entry.path(), output)?;
        }
    }
    Ok(())
}
