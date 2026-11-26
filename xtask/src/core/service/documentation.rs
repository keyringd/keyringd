use crate::core::{driver::PromoVideoRenderer, model::Workspace};
use anyhow::{Context, Result, ensure};
use std::process::Command;
pub(crate) fn build_website(workspace: &Workspace, base_path: &str) -> Result<()> {
    let mut process = website_command(workspace);
    process
        .env("QUARKUS_ROQ_GENERATOR_BATCH", "true")
        .args(["--no-daemon", "clean", "quarkusRun", "-x", "test"])
        .arg(format!("-Dquarkus.http.root-path={base_path}"));
    run_website(process)?;
    let index = workspace.root().join("website/build/roq/index.html");
    ensure!(index.is_file(), "Roq did not generate {}", index.display());
    println!("generated {}", index.display());
    Ok(())
}

pub(crate) fn serve_website(workspace: &Workspace, port: u16) -> Result<()> {
    let mut process = website_command(workspace);
    process
        .arg("quarkusDev")
        .arg(format!("-Dquarkus.http.port={port}"));
    run_website(process)
}

pub(crate) fn build_promo_video(workspace: &Workspace) -> Result<()> {
    PromoVideoRenderer::new(workspace).build()?;
    let video = workspace
        .root()
        .join("website/public/media/keyringd-in-60-seconds.mp4");
    ensure!(
        video.is_file(),
        "promo video was not generated: {}",
        video.display()
    );
    println!("generated {}", video.display());
    Ok(())
}

fn website_command(workspace: &Workspace) -> Command {
    let website = workspace.root().join("website");
    let mut process = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/d", "/c", "gradlew.bat"]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("./gradlew");
        cmd
    };
    process.current_dir(&website);
    process
}

fn run_website(mut process: Command) -> Result<()> {
    let status = process
        .status()
        .context("could not start the website Gradle Wrapper; install JDK 21 or newer")?;
    ensure!(status.success(), "Roq exited with {status}");
    Ok(())
}
