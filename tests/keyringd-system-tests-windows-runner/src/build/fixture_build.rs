use super::FixtureExecutables;

use crate::environment::Workspace;
use anyhow::{Result, ensure};
use std::path::PathBuf;

pub(crate) fn build_fixture(workspace: &Workspace) -> Result<FixtureExecutables> {
    let app = build_command(workspace, "build")
        .args([
            "-p",
            "keyringd-windows",
            "--features",
            "test-support",
            "--bin",
            "keyringd-windows-ui-test-app",
            "--message-format=json",
        ])
        .output()?;
    let app = executable(app, "keyringd-windows-ui-test-app")?;
    let security = build_command(workspace, "build")
        .args([
            "-p",
            "keyringd-system-tests",
            "--features",
            "test-support",
            "--bin",
            "keyringd-windows-security-automation",
            "--message-format=json",
        ])
        .output()?;
    Ok(FixtureExecutables {
        app,
        security_automation: executable(security, "keyringd-windows-security-automation")?,
    })
}
pub(crate) fn build_scenarios(workspace: &Workspace, package: &str) -> Result<PathBuf> {
    let output = build_command(workspace, "test")
        .args([
            "-p",
            package,
            "--test",
            "scenarios",
            "--no-run",
            "--message-format=json",
        ])
        .output()?;
    executable(output, "scenarios")
}

fn executable(output: std::process::Output, target: &str) -> Result<PathBuf> {
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

fn build_command(workspace: &Workspace, action: &str) -> std::process::Command {
    let mut command =
        std::process::Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .args([
            "--config",
            "net.git-fetch-with-cli=false",
            action,
            "--locked",
            "--manifest-path",
        ])
        .arg(workspace.root.join("Cargo.toml"))
        .current_dir(&workspace.root);
    command
}
