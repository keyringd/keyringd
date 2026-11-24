use anyhow::{Context, Result, ensure};
use std::process::Command;

pub(crate) fn run_command(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to run: {:?}", command.get_program()))?;
    ensure!(
        status.success(),
        "{:?} exited with {status}",
        command.get_program()
    );
    Ok(())
}
