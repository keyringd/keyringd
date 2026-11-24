use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use uuid::Uuid;

use crate::application::platform::linux::atspi::AtspiSession;
use crate::environment::linux::Environment;

pub(super) async fn capture(driver: &AtspiSession, environment: &Environment) -> Result<PathBuf> {
    let (path, report_path) = destination(environment)?;
    let bounds = driver.capture_extents().await?;
    run_scrot(
        environment,
        &path,
        &[
            "--autoselect",
            &format!(
                "{},{},{},{}",
                bounds.x, bounds.y, bounds.width, bounds.height
            ),
        ],
    )
    .await?;
    Ok(report_path)
}

pub(super) async fn read_hybrid_qr(
    driver: &AtspiSession,
    environment: &Environment,
) -> Result<(String, PathBuf)> {
    let report_path = capture(driver, environment).await?;
    let screenshot = environment.artifact(
        report_path
            .strip_prefix("runtime")
            .expect("capture should return a report path under runtime because destination constructs it with that prefix"),
    );
    let qr = keyringd_testkit::QrCodeReader::read_hybrid(&screenshot)?
        .context("captured window does not contain a Hybrid QR")?;
    Ok((qr, report_path))
}

fn destination(environment: &Environment) -> Result<(PathBuf, PathBuf)> {
    let relative_path = Path::new("screenshots").join(format!("{}.png", Uuid::new_v4()));
    let path = environment.artifact(&relative_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("failed to create screenshot directory {}", parent.display())
        })?;
    }
    Ok((path, Path::new("runtime").join(relative_path)))
}

async fn run_scrot(environment: &Environment, path: &Path, selection: &[&str]) -> Result<()> {
    let mut arguments = vec!["--silent", "--overwrite"];
    arguments.extend_from_slice(selection);
    let path_argument = path.to_string_lossy();
    arguments.push(path_argument.as_ref());
    let output = environment.run("scrot", &arguments).await?;
    ensure!(output.status.success(), "scrot failed: {}", output.stderr);
    ensure!(
        std::fs::metadata(path).is_ok_and(|metadata| metadata.len() > 0),
        "scrot did not create a non-empty screenshot at {}",
        path.display()
    );
    Ok(())
}
