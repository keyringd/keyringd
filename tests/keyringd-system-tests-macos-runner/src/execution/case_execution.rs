use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Result, ensure};

use super::{ProcessOutcome, case_runner};

/// Always records the case after supervision, diagnostics and cleanup attempts.
pub(super) struct CaseExecution<'a> {
    report_directory: &'a Path,
    source: &'a Path,
    directory: PathBuf,
    id: String,
    timeout: Duration,
}

impl<'a> CaseExecution<'a> {
    pub(super) fn new(
        report_directory: &'a Path,
        source: &'a Path,
        directory: PathBuf,
        id: &str,
        timeout: Duration,
    ) -> Self {
        Self {
            report_directory,
            source,
            directory,
            id: id.to_owned(),
            timeout,
        }
    }

    pub(super) fn run(
        &self,
        command: &mut Command,
        cleanup: impl FnOnce(&Path) -> Result<Option<String>>,
    ) -> Result<bool> {
        let started = std::time::Instant::now();
        let supervised = case_runner::supervise(command, &self.directory, self.timeout);
        let elapsed = started.elapsed().as_millis();
        let mut failure = match supervised {
            Ok((ProcessOutcome::Exited(status), _)) if status.success() => None,
            Ok((ProcessOutcome::Exited(status), _)) => {
                Some(format!("scenario process exited with {status}"))
            }
            Ok((ProcessOutcome::TimedOut, _)) => Some(format!(
                "case deadline exceeded ({} seconds)",
                self.timeout.as_secs()
            )),
            Err(error) => Some(format!("case supervisor failed: {error:#}")),
        };
        let cleanup = cleanup(&self.directory);
        let cleanup_failed = cleanup.is_err();
        let cleanup_message = match cleanup {
            Ok(message) => message,
            Err(error) => Some(format!("case cleanup failed: {error:#}")),
        };
        if let Some(message) = cleanup_message {
            failure = Some(match failure {
                Some(previous) => format!("{previous}\n{message}"),
                None => message,
            });
        }
        let mut artifacts = match case_runner::artifacts(&self.directory) {
            Ok(artifacts) => artifacts,
            Err(error) => {
                let message = format!("case evidence collection failed: {error:#}");
                failure = Some(match failure {
                    Some(previous) => format!("{previous}\n{message}"),
                    None => message,
                });
                Vec::new()
            }
        };
        if failure.is_none() {
            // An expected startup/restart error may have saved diagnostics before
            // the scenario recovered. Keep those out of a successful report.
            let diagnostics = self.directory.join("diagnostics");
            artifacts.retain(|path| !path.starts_with(&diagnostics));
        }
        let server_log = self
            .report_directory
            .join("authenticator-server/authenticator.log");
        if server_log.is_file() {
            artifacts.push(server_log);
        }
        if self.directory.join("authenticator-residue.json").exists() {
            artifacts.push(self.directory.join("authenticator-residue.json"));
        }
        let passed = keyringd_testkit::supervision::finish_case(
            self.report_directory,
            &self.id,
            &self.directory.join("checkpoint.json"),
            self.source,
            failure,
            elapsed,
            &artifacts,
        )
        .map_err(anyhow::Error::from_boxed)?;
        ensure!(
            !cleanup_failed,
            "case cleanup failed after {}; case failure was recorded",
            self.id
        );
        Ok(passed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_system_tests::environment::macos::diagnostics::FixtureRecord;

    fn assert_failure(report: &Path, text: &str) {
        let paths: Vec<_> = std::fs::read_dir(report)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .is_some_and(|extension| extension == "json")
            })
            .collect();
        assert_eq!(paths.len(), 1, "expected one case result: {paths:?}");
        let result: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&paths[0]).unwrap()).unwrap();
        assert_eq!(result["outcome"], "failed");
        assert!(
            result["failure"].as_str().unwrap().contains(text),
            "{result}"
        );
        keyringd_testkit::generate_html_report(report).unwrap();
        let html = std::fs::read_to_string(report.join("index.html")).unwrap();
        assert!(html.contains(text));
        assert!(html.contains("scenario.log"));
    }

    keyringd_testkit::scenario! {
        group "CaseExecution::run" {
            case "Spawn failure is recorded with its log" {
                verify "save the process launch error and associate its log with the failed case" {
                    let report = tempfile::tempdir().unwrap();
                    let source = report.path().join("case.rs");
                    std::fs::write(&source, "source").unwrap();
                    let case = CaseExecution::new(
                        report.path(),
                        &source,
                        report.path().join("cases/1"),
                        "keyringd-system-tests::scenarios::spawn",
                        Duration::from_secs(1),
                    );
                    assert!(
                        !case
                            .run(
                                &mut Command::new(report.path().join("missing-executable")),
                                |_| Ok(None)
                            )
                            .unwrap()
                    );
                    assert_failure(report.path(), "failed to start case process");
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CaseExecution::run" {
            case "Failed case preserves rpc diagnostics and removes private state" {
                verify "retain RPC diagnostics while removing the failed case's private state" {
                    let report = tempfile::tempdir().unwrap();
                    let source = report.path().join("case.rs");
                    std::fs::write(&source, "source").unwrap();
                    let directory = report.path().join("cases/1");
                    std::fs::create_dir_all(directory.join("fixtures")).unwrap();
                    let fixture = tempfile::Builder::new()
                        .prefix("p4s-ui-")
                        .tempdir_in(case_runner::fixture_root())
                        .unwrap();
                    std::fs::write(fixture.path().join("app.log"), "RPC failure fixture").unwrap();
                    // Invalid endpoint: both diagnostic failures must be retained, without
                    // requiring a GUI or a listening process.
                    std::fs::write(fixture.path().join("control-address"), "invalid endpoint").unwrap();
                    std::fs::write(
                        directory.join("fixtures/app.json"),
                        serde_json::to_vec(&FixtureRecord {
                            directory: fixture.path().into(),
                        })
                        .unwrap(),
                    )
                    .unwrap();
                    let case = CaseExecution::new(
                        report.path(),
                        &source,
                        directory.clone(),
                        "keyringd-system-tests::scenarios::rpc",
                        Duration::from_secs(1),
                    );
                    assert!(
                        !case
                            .run(Command::new("sh").args(["-c", "exit 7"]), |_| Ok(None))
                            .unwrap()
                    );
                    assert!(!fixture.path().exists());
                    assert_failure(report.path(), "scenario process exited");
                    let html = std::fs::read_to_string(report.path().join("index.html")).unwrap();
                    assert!(html.contains("ax-tree.txt"));
                    assert!(html.contains("screenshot-error.txt"));
                    assert!(html.contains("app.log"));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CaseExecution::run" {
            case "Cleanup failure records the case before aborting" {
                verify "write the case result before surfacing the fixture cleanup failure" {
                    let report = tempfile::tempdir().unwrap();
                    let source = report.path().join("case.rs");
                    std::fs::write(&source, "source").unwrap();
                    let case = CaseExecution::new(
                        report.path(),
                        &source,
                        report.path().join("cases/1"),
                        "keyringd-system-tests::scenarios::cleanup",
                        Duration::from_secs(1),
                    );
                    let error = case
                        .run(Command::new("sh").args(["-c", "exit 0"]), |directory| {
                            std::fs::write(directory.join("authenticator-residue.json"), "[]")?;
                            anyhow::bail!("injected cleanup failure")
                        })
                        .unwrap_err();
                    assert!(error.to_string().contains("case failure was recorded"));
                    assert_failure(report.path(), "injected cleanup failure");
                    let html = std::fs::read_to_string(report.path().join("index.html")).unwrap();
                    assert!(html.contains("authenticator-residue.json"));
                }
            }
        }
    }
}
