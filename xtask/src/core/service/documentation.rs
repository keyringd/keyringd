use crate::core::model::Workspace;
use anyhow::{Context, Result, ensure};
use std::process::Command;
pub(crate) fn build_website(workspace: &Workspace, base_path: &str) -> Result<()> {
    let mut process = website_command(workspace);
    process
        .env("QUARKUS_ROQ_GENERATOR_BATCH", "true")
        .args([
            "--batch-mode",
            "clean",
            "package",
            "quarkus:run",
            "-DskipTests",
        ])
        .arg(format!("-Dquarkus.http.root-path={base_path}"));
    run_website(process)?;
    let index = workspace.root().join("website/target/roq/index.html");
    ensure!(index.is_file(), "Roq did not generate {}", index.display());
    println!("generated {}", index.display());
    Ok(())
}

pub(crate) fn serve_website(workspace: &Workspace, port: u16) -> Result<()> {
    let mut process = website_command(workspace);
    process
        .arg("quarkus:dev")
        .arg(format!("-Dquarkus.http.port={port}"));
    run_website(process)
}

fn website_command(workspace: &Workspace) -> Command {
    let website = workspace.root().join("website");
    let mut process = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/d", "/c", "mvnw.cmd"]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("./mvnw");
        cmd
    };
    process.current_dir(&website);
    process
}

fn run_website(mut process: Command) -> Result<()> {
    let status = process
        .status()
        .context("could not start the website Maven Wrapper; install JDK 21 or newer")?;
    ensure!(status.success(), "Roq exited with {status}");
    Ok(())
}
