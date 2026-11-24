use anyhow::{Context, Result, bail};
use std::process::{Command, Output};

pub(crate) fn checked_output(command: &mut Command, description: &str) -> Result<Output> {
    let output = command
        .output()
        .with_context(|| format!("failed to start {description}"))?;
    if !output.status.success() {
        bail!(
            "{description} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output)
}
