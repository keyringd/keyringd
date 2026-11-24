use std::ffi::OsString;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};

use crate::environment::testbed::TestbedExecutionRequest;
use crate::runner_options::RunnerOptions;
use crate::util::command;

pub(crate) fn prepare(options: &RunnerOptions) -> Result<TestbedExecutionRequest> {
    if options.package.is_none() {
        command::require_success(
            "keyringd release build",
            cargo_executable(),
            ["build", "--release", "--package", "keyringd-linux"],
            &options.workspace_root,
        )?;
    }

    let package = options
        .package
        .as_ref()
        .map(|path| workspace_path(&options.workspace_root, path))
        .transpose()?
        .map(|path| relative_to(&options.workspace_root, &path))
        .transpose()?;

    let test_executable = build_test_executable(options, "keyringd-system-tests")?;

    let runtime_directory = options.report_directory.join("runtime");
    fs::create_dir_all(&runtime_directory).with_context(|| {
        format!(
            "failed to create E2E report directory {}",
            options.report_directory.display()
        )
    })?;
    fs::File::create(runtime_directory.join("nspawn.log"))
        .context("failed to prepare the systemd-nspawn log")?;

    let image_directory = options.workspace_root.join("target/linux-system/images");
    fs::create_dir_all(&image_directory).with_context(|| {
        format!(
            "failed to create Linux system test image cache {}",
            image_directory.display()
        )
    })?;
    let image_definition = options
        .workspace_root
        .join("tests/keyringd-system-tests-linux-runner/testbed/mkosi")
        .join(options.distribution.id());
    let image_definition_hash = directory_hash(&image_definition)?;

    Ok(TestbedExecutionRequest {
        distribution: options.distribution,
        workspace: options.workspace_root.clone(),
        package,
        test_executable: relative_to(&options.workspace_root, &test_executable)?,
        report_directory: relative_to(&options.workspace_root, &options.report_directory)?,
        image: image_directory.join(options.distribution.image_file_name(&image_definition_hash)),
        machine: format!("keyringd-e2e-{}", std::process::id()),
        owner_uid: numeric_id(options, "-u")?,
        owner_gid: numeric_id(options, "-g")?,
        authenticator_control_url: crate::environment::testbed::AUTHENTICATOR_CONTROL_URL
            .to_owned(),
        filter: options.filter.clone(),
    })
}

fn directory_hash(directory: &Path) -> Result<String> {
    let mut files = Vec::new();
    collect_files(directory, &mut files)?;
    files.sort();

    let mut hasher = Sha256::new();
    for path in files {
        let relative = path.strip_prefix(directory).with_context(|| {
            format!(
                "{} is outside image definition {}",
                path.display(),
                directory.display()
            )
        })?;
        hasher.update(relative.to_string_lossy().as_bytes());
        hasher.update([0]);
        hasher.update(
            fs::read(&path)
                .with_context(|| format!("failed to read image definition {}", path.display()))?,
        );
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    Ok(digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory).with_context(|| {
        format!(
            "failed to read Linux system test image definition {}",
            directory.display()
        )
    })? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn workspace_path(root: &Path, path: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let path = path
        .canonicalize()
        .with_context(|| format!("E2E package is missing: {}", path.display()))?;
    ensure!(
        path.starts_with(root),
        "E2E package must be inside workspace {}: {}",
        root.display(),
        path.display()
    );
    Ok(path)
}

fn relative_to(root: &Path, path: &Path) -> Result<PathBuf> {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .with_context(|| format!("{} is outside workspace {}", path.display(), root.display()))
}

fn numeric_id(options: &RunnerOptions, option: &str) -> Result<u32> {
    command::output("id", [option], &options.workspace_root)?
        .trim()
        .parse()
        .with_context(|| format!("id {option} did not return a numeric identifier"))
}

fn build_test_executable(options: &RunnerOptions, package: &str) -> Result<PathBuf> {
    let mut child = Command::new(cargo_executable())
        .args([
            "test",
            "--package",
            package,
            "--test",
            "scenarios",
            "--no-run",
            "--message-format=json",
        ])
        .current_dir(&options.workspace_root)
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to start the Linux system test build")?;
    let stdout = child
        .stdout
        .take()
        .context("Cargo stdout was not captured")?;
    let mut artifacts = keyringd_testkit::runner::CargoBuildArtifacts::default();
    for line in BufReader::new(stdout).lines() {
        let line = line.context("failed to read Cargo build output")?;
        if let Some(diagnostic) = artifacts.push_line(&line)? {
            eprint!("{diagnostic}");
        }
    }
    let status = child
        .wait()
        .context("failed to wait for the Cargo test build")?;
    ensure!(
        status.success(),
        "Linux system test build failed with {status}"
    );
    let executable = artifacts.executable("scenarios")?;
    if !executable.is_file() {
        bail!(
            "Cargo reported a missing E2E scenarios executable: {}",
            executable.display()
        );
    }
    Ok(executable)
}

fn cargo_executable() -> OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}
