use super::UiTestAppBundle;

use crate::environment::Workspace;
use anyhow::Result;
use std::path::PathBuf;

pub(crate) const FIXTURE: &str = "keyringd-macos-ui-test-app";
const SCENARIOS: &str = "scenarios";

pub(crate) fn build_fixture(workspace: &Workspace) -> Result<PathBuf> {
    let output = std::process::Command::new(workspace.cargo())
        .args([
            "build",
            "-p",
            "keyringd-macos",
            "--features",
            "test-support",
            "--bin",
            FIXTURE,
            "--message-format=json",
        ])
        .current_dir(workspace.root())
        .output()?;
    let binary = executable(output, FIXTURE)?;
    UiTestAppBundle::prepare(workspace.root(), &binary)
}

/// Builds the scenario executable and returns its path, so it can be run from
/// the workspace root instead of the crate directory.
pub(crate) fn build_scenarios(workspace: &Workspace) -> Result<PathBuf> {
    use anyhow::Context;

    let output = std::process::Command::new(workspace.cargo())
        .args([
            "test",
            "-p",
            "keyringd-system-tests",
            "--test",
            SCENARIOS,
            "--no-run",
            "--message-format=json",
        ])
        .current_dir(workspace.root())
        .output()
        .context("failed to execute cargo test --no-run")?;
    executable(output, SCENARIOS)
}

fn executable(output: std::process::Output, target: &str) -> Result<PathBuf> {
    use anyhow::ensure;
    let mut artifacts = keyringd_testkit::runner::CargoBuildArtifacts::default();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(diagnostic) = artifacts.push_line(line)? {
            eprint!("{diagnostic}");
        }
    }
    ensure!(
        output.status.success(),
        "Cargo build failed for {target}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    artifacts.executable(target)
}
