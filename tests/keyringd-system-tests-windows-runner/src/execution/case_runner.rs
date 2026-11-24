use crate::RunnerOptions;
#[cfg(windows)]
pub(super) fn configure_environment(
    command: &mut std::process::Command,
    options: &RunnerOptions,
    fixture: &std::path::Path,
    openssh: Option<&std::path::Path>,
    authenticator: &crate::environment::AuthenticatorEnvironment,
) {
    command
        .env("KEYRINGD_TEST_REPORT_DIR", &options.report_directory)
        .env("KEYRINGD_UI_TEST_APP", fixture)
        .env(
            "KEYRINGD_TEST_USBIP_SERVER",
            authenticator.usb_address.to_string(),
        )
        .env(
            "KEYRINGD_TEST_AUTHENTICATOR_CONTROL",
            &authenticator.control_url,
        )
        .env(
            "KEYRINGD_UI_TEST_HYBRID_TUNNEL_ADDRESS",
            authenticator.hybrid_address.to_string(),
        )
        .env(
            "KEYRINGD_UI_TEST_HYBRID_TUNNEL_CA",
            &authenticator.hybrid_ca,
        );
    if let Some(openssh) = openssh {
        command.env("KEYRINGD_TEST_OPENSSH", openssh);
    } else {
        command.env_remove("KEYRINGD_TEST_OPENSSH");
    }
}

/// Scenario actors inherit the runner's user and privileges.
pub(super) fn run(
    command: &mut std::process::Command,
    options: &RunnerOptions,
    case: &str,
    fixture: &std::path::Path,
) -> anyhow::Result<bool> {
    use anyhow::{Context, Result};
    use keyringd_testkit::runner::ScenarioResults;
    use std::process::Stdio;
    use std::time::{Duration, Instant};
    let (crate_name, source) = match options.suite {
        crate::TestSuite::Windows => (
            "keyringd-system-tests",
            "tests/keyringd-system-tests/tests/main.rs",
        ),
        crate::TestSuite::Wsl { .. } => (
            "keyringd-wsl-system-tests",
            "tests/keyringd-wsl-system-tests/tests/scenarios/main.rs",
        ),
    };
    let prefix = format!("{crate_name}::scenarios::");
    let id = format!("{prefix}{case}");
    let case_id = uuid::Uuid::new_v4().to_string();
    let directory = options.report_directory.join("cases").join(&case_id);
    std::fs::create_dir_all(&directory)?;
    let log_path = directory.join("scenario.log");
    let log = std::fs::File::create(&log_path)?;
    let checkpoint = directory.join("checkpoint.json");
    command
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .env("KEYRINGD_UI_TEST_TRAY_GUID", &case_id)
        .env("KEYRINGD_UI_CASE_DIR", &directory)
        .env("KEYRINGD_TEST_CASE_CHECKPOINT", &checkpoint);
    let started = Instant::now();
    let (outcome, process_cleanup) = supervise(command, Duration::from_secs(300));
    let mut failure = match outcome {
        Ok(status) if status.success() => None,
        Ok(status) => Some(format!("scenario process exited with {status}")),
        Err(error) => Some(format!("scenario supervision failed: {error:#}")),
    };
    let key_cleanup: Result<()> = (|| {
        let ledgers = directory.join("device-keys");
        let mut stages = Vec::new();
        if ledgers.try_exists()? {
            for entry in std::fs::read_dir(ledgers)? {
                let entry = entry?;
                stages.push((
                    entry.path().display().to_string(),
                    keyringd_system_tests::environment::windows::TestApp::cleanup_device_keys(
                        fixture,
                        &entry.path(),
                    ),
                ));
            }
        }
        keyringd_testkit::runner::StageFailures::finish(stages)
    })();
    let cleanup = keyringd_testkit::runner::StageFailures::finish([
        ("scenario process cleanup", process_cleanup),
        ("case device-key cleanup", key_cleanup),
    ]);
    let cleanup_failed = cleanup.is_err();
    if let Err(error) = cleanup {
        let cleanup = format!("case cleanup failed: {error:#}");
        failure = Some(failure.map_or_else(
            || cleanup.clone(),
            |previous| format!("{previous}\n{cleanup}"),
        ));
    }
    let recorded = ScenarioResults::load(&options.report_directory, &prefix)?.contains(case);
    if failure.is_none() && recorded {
        return Ok(true);
    } // Preserve the suite's existing skip policy.
    let passed = keyringd_testkit::supervision::finish_case(
        &options.report_directory,
        &id,
        &checkpoint,
        &options.workspace_root.join(source),
        failure,
        started.elapsed().as_millis(),
        &[log_path],
    )
    .map_err(anyhow::Error::from_boxed)
    .context("failed to record supervised scenario result")?;
    anyhow::ensure!(
        !cleanup_failed,
        "case cleanup failed after {id}; failure was recorded and the run must stop"
    );
    Ok(passed)
}

fn supervise(
    command: &mut std::process::Command,
    timeout: std::time::Duration,
) -> (anyhow::Result<std::process::ExitStatus>, anyhow::Result<()>) {
    let mut process = match super::case_process::CaseProcess::spawn(command) {
        Ok(process) => process,
        Err(error) => return (Err(error), Ok(())),
    };
    let started = std::time::Instant::now();
    let outcome = (|| {
        loop {
            if let Some(status) = process.child.try_wait()? {
                return Ok(status);
            }
            anyhow::ensure!(
                started.elapsed() < timeout,
                "case deadline exceeded ({} seconds)",
                timeout.as_secs()
            );
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    })();
    (outcome, process.stop())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    keyringd_testkit::scenario! {
        group "Windows case supervision" {
            case "A hung scenario is terminated and reaped at the case deadline" {
                verify "the supervisor owns the process on the timeout path" {
                    let mut command = Command::new("powershell.exe");
                    command.args(["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 30"]);
                    let (outcome, cleanup) = supervise(&mut command, std::time::Duration::from_millis(10));
                    cleanup.unwrap();
                    let error = outcome.unwrap_err();
                    assert!(format!("{error:#}").contains("case deadline exceeded"));
                }
            }
            case "A scenario exiting without a result is recorded as failed" {
                verify "the report preserves the abnormal exit and scenario log" {
                    let root = tempfile::tempdir().unwrap();
                    let options = RunnerOptions { suite: crate::TestSuite::Windows, workspace_root: root.path().to_owned(), report_directory: root.path().join("report"), filter: None };
                    let mut command = Command::new("cmd.exe");
                    command.args(["/C", "exit", "/B", "3"]);
                    assert!(!run(&mut command, &options, "crashed", &std::env::current_exe().unwrap()).unwrap());
                    let results = keyringd_testkit::runner::ScenarioResults::load(&options.report_directory, "keyringd-system-tests::scenarios::").unwrap();
                    assert!(results.contains("crashed"));
                    assert_eq!(results.failed(), 1);
                    let result = std::fs::read_dir(&options.report_directory).unwrap().map(|entry| entry.unwrap().path()).find(|path| path.extension().is_some_and(|extension| extension == "json")).unwrap();
                    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(result).unwrap()).unwrap();
                    assert!(value["failure"].as_str().unwrap().contains("scenario process exited"));
                    assert!(!value["evidence"].as_array().unwrap().is_empty());
                }
            }
        }
    }
}
