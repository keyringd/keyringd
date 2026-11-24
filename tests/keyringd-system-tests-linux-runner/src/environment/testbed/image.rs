use std::fs;

use anyhow::{Context, Result, ensure};

use super::TestbedExecutionRequest;
use crate::util::command;

pub(super) fn prepare(request: &TestbedExecutionRequest) -> Result<()> {
    let definition = request
        .workspace
        .join("tests/keyringd-system-tests-linux-runner/testbed/mkosi")
        .join(request.distribution.id());
    ensure!(
        definition.join("mkosi.conf").is_file(),
        "mkosi image definition is missing: {}",
        definition.display()
    );
    if request.image.is_file() {
        return Ok(());
    }

    let parent = request
        .image
        .parent()
        .context("Linux system test image path has no parent directory")?;
    fs::create_dir_all(parent).with_context(|| {
        format!(
            "failed to create Linux system test image cache {}",
            parent.display()
        )
    })?;

    command::require_success(
        &format!("{} Linux system test image build", request.distribution),
        "mkosi",
        [
            "--directory".as_ref(),
            definition.as_os_str(),
            "--release".as_ref(),
            request.distribution.release().as_ref(),
            "--output-dir".as_ref(),
            parent.as_os_str(),
            "--output".as_ref(),
            request
                .image
                .file_stem()
                .context("Linux system test image path has no file stem")?,
            "build".as_ref(),
        ],
        &request.workspace,
    )?;
    ensure!(
        request.image.is_file(),
        "mkosi did not produce {}",
        request.image.display()
    );
    Ok(())
}
