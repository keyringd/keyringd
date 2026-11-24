//! One process group per case, supervised from outside the scenario process.
use super::{case_process::CaseProcess, process_outcome::ProcessOutcome};
use anyhow::{Context, Result};
use keyringd_system_tests::environment::macos::diagnostics::FixtureRecord;
use keyringd_system_tests::environment::macos::diagnostics::collect;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(super) fn supervise(
    command: &mut Command,
    directory: &Path,
    timeout: Duration,
) -> Result<(ProcessOutcome, u128)> {
    std::fs::create_dir_all(directory)?;
    let log = std::fs::File::create(directory.join("scenario.log"))?;
    command
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .env("KEYRINGD_UI_CASE_DIR", directory)
        .env(
            "KEYRINGD_TEST_CASE_CHECKPOINT",
            directory.join("checkpoint.json"),
        );
    let mut process = CaseProcess::spawn(command)?;
    let started = Instant::now();
    let outcome = loop {
        if let Some(status) = process.child.try_wait()? {
            break ProcessOutcome::Exited(status);
        }
        if started.elapsed() >= timeout {
            break ProcessOutcome::TimedOut;
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    // Collect while the fixture is alive. Each diagnostic RPC is independently
    // bounded, even if AppKit is stuck. Normal panic diagnostics survive Drop.
    let diagnostics = if matches!(&outcome, ProcessOutcome::Exited(status) if status.success()) {
        Ok(())
    } else {
        collect_registered_fixtures(directory)
    };
    let elapsed = started.elapsed().as_millis();
    drop(process); // also kills orphan descendants after ordinary success/failure
    let cleanup = remove_registered_state(directory);
    diagnostics?;
    cleanup?;
    Ok((outcome, elapsed))
}

fn fixture_records(directory: &Path) -> Result<Vec<FixtureRecord>> {
    let records = directory.join("fixtures");
    if !records.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(records)? {
        let path = entry?.path();
        out.push(serde_json::from_slice(&std::fs::read(path)?)?);
    }
    Ok(out)
}

fn collect_registered_fixtures(directory: &Path) -> Result<()> {
    for record in fixture_records(directory)? {
        // Already collected during panic, or removed by ordinary RAII cleanup.
        if !record.directory.is_dir() {
            continue;
        }
        let name = record
            .directory
            .file_name()
            .context("fixture record has no name")?;
        let destination = directory.join("diagnostics").join(name);
        collect(&record.directory, &destination)?;
    }
    Ok(())
}

fn remove_registered_state(directory: &Path) -> Result<()> {
    for record in fixture_records(directory)? {
        // Only directories made by our fixture's tempfile builder are recorded.
        // Keep an explicit guard before recursive removal after a forced exit.
        anyhow::ensure!(
            record.directory.parent() == Some(fixture_root().as_path())
                && record
                    .directory
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("p4s-ui-")),
            "invalid private fixture directory: {}",
            record.directory.display()
        );
        if record.directory.exists() {
            std::fs::remove_dir_all(record.directory)?;
        }
    }
    Ok(())
}

pub(super) fn fixture_root() -> PathBuf {
    // Match the product UI fixture's short Unix socket paths on macOS.
    if cfg!(target_os = "macos") {
        PathBuf::from("/tmp")
    } else {
        std::env::temp_dir()
    }
}

pub(super) fn artifacts(directory: &Path) -> Result<Vec<PathBuf>> {
    fn walk(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
        for entry in std::fs::read_dir(path)? {
            let path = entry?.path();
            if path.is_dir() {
                walk(&path, out)?;
            } else {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut out = vec![directory.join("scenario.log")];
    let diagnostics = directory.join("diagnostics");
    if diagnostics.is_dir() {
        walk(&diagnostics, &mut out)?;
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "supervise" {
            case "Successful case does not collect fixture diagnostics" {
                verify "skip fixture diagnostic collection on a successful result" {
                    let directory = tempfile::tempdir().unwrap();
                    std::fs::create_dir(directory.path().join("fixtures")).unwrap();
                    let fixture = tempfile::Builder::new()
                        .prefix("p4s-ui-")
                        .tempdir_in(fixture_root())
                        .unwrap();
                    std::fs::write(fixture.path().join("app.log"), "successful fixture").unwrap();
                    std::fs::write(fixture.path().join("control-address"), "invalid endpoint").unwrap();
                    std::fs::write(
                        directory.path().join("fixtures/app.json"),
                        serde_json::to_vec(&FixtureRecord {
                            directory: fixture.path().into(),
                        })
                        .unwrap(),
                    )
                    .unwrap();
                    let (outcome, _) = supervise(
                        Command::new("sh").args(["-c", "exit 0"]),
                        directory.path(),
                        Duration::from_secs(1),
                    )
                    .unwrap();
                    assert!(matches!(outcome, ProcessOutcome::Exited(status) if status.success()));
                    assert!(!directory.path().join("diagnostics").exists());
                    assert!(!fixture.path().exists());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "supervise" {
            case "Times out and kills its descendants" {
                verify "terminate the fixture and its child processes after the deadline" {
                    let directory = tempfile::tempdir().unwrap();
                    let mut command = Command::new("sh");
                    // Record the descendant PID before waiting so Linux can verify its death.
                    command.args(["-c", "sleep 30 & echo $!; echo started; wait"]);
                    let started = Instant::now();
                    let (outcome, _) =
                        supervise(&mut command, directory.path(), Duration::from_millis(100)).unwrap();
                    assert!(matches!(outcome, ProcessOutcome::TimedOut));
                    assert!(started.elapsed() < Duration::from_secs(5));
                    let log = std::fs::read_to_string(directory.path().join("scenario.log")).unwrap();
                    assert!(log.contains("started"));
                    #[cfg(target_os = "linux")]
                    {
                        let pid: u32 = log.lines().next().unwrap().parse().unwrap();
                        let deadline = Instant::now() + Duration::from_secs(1);
                        loop {
                            let state = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok();
                            if state
                                .as_ref()
                                .is_none_or(|s| s.split_once(") ").unwrap().1.starts_with('Z'))
                            {
                                break;
                            }
                            assert!(
                                Instant::now() < deadline,
                                "descendant survived case termination: {state:?}"
                            );
                            std::thread::sleep(Duration::from_millis(10));
                        }
                    }
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "supervise" {
            case "Preserves a normal exit status" {
                verify "return the original exit status of a process that finishes in time" {
                    let directory = tempfile::tempdir().unwrap();
                    let (outcome, _) = supervise(
                        Command::new("sh").args(["-c", "exit 7"]),
                        directory.path(),
                        Duration::from_secs(5),
                    )
                    .unwrap();
                    assert!(matches!(outcome, ProcessOutcome::Exited(status) if status.code() == Some(7)));
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "supervise" {
            case "A timed out fixture is collected cleaned and linked in html" {
                verify "attach timeout diagnostics to the HTML result and remove the fixture state" {
                    use keyringd_testkit::macos_ui_protocol::{
                        ControlCommand, ControlResponse, UiCommand, UiResponse,
                    };
                    use std::io::{BufRead, BufReader, Write};
                    use std::net::TcpListener;
                    let report = tempfile::tempdir().unwrap();
                    let directory = report.path().join("cases/0001");
                    std::fs::create_dir_all(directory.join("fixtures")).unwrap();
                    let fixture = tempfile::Builder::new()
                        .prefix("p4s-ui-")
                        .tempdir_in(fixture_root())
                        .unwrap();
                    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
                    std::fs::write(
                        fixture.path().join("control-address"),
                        listener.local_addr().unwrap().to_string(),
                    )
                    .unwrap();
                    std::fs::write(fixture.path().join("app.log"), "fixture log").unwrap();
                    std::fs::write(fixture.path().join("profiles.json"), "[]").unwrap();
                    std::fs::write(
                        directory.join("fixtures/fixture.json"),
                        serde_json::to_vec(&FixtureRecord {
                            directory: fixture.path().into(),
                        })
                        .unwrap(),
                    )
                    .unwrap();
                    let server = std::thread::spawn(move || {
                        for _ in 0..2 {
                            let (mut stream, _) = listener.accept().unwrap();
                            stream
                                .set_read_timeout(Some(Duration::from_secs(2)))
                                .unwrap();
                            let mut request = String::new();
                            BufReader::new(stream.try_clone().unwrap())
                                .read_line(&mut request)
                                .unwrap();
                            let request: ControlCommand = serde_json::from_str(&request).unwrap();
                            let result = match request.ui.unwrap() {
                                UiCommand::Tree => UiResponse::Tree {
                                    tree: "AXWindow fixture".into(),
                                },
                                UiCommand::Screenshot { path, .. } => {
                                    std::fs::write(&path, b"PNG fixture image").unwrap();
                                    UiResponse::Screenshot { path }
                                }
                                _ => panic!("unexpected diagnostic request"),
                            };
                            writeln!(
                                stream,
                                "{}",
                                serde_json::to_string(&ControlResponse::Ui { result }).unwrap()
                            )
                            .unwrap();
                        }
                    });
                    let (outcome, elapsed) = supervise(
                        Command::new("sleep").arg("30"),
                        &directory,
                        Duration::from_millis(100),
                    )
                    .unwrap();
                    server.join().unwrap();
                    assert!(matches!(outcome, ProcessOutcome::TimedOut));
                    assert!(!fixture.path().exists());
                    let evidence = artifacts(&directory).unwrap();
                    assert!(evidence.iter().any(|p| p.ends_with("ax-tree.txt")));
                    assert!(evidence.iter().any(|p| p.ends_with("window.png")));
                    let source = report.path().join("case.rs");
                    std::fs::write(&source, "source").unwrap();
                    assert!(
                        !keyringd_testkit::supervision::finish_case(
                            report.path(),
                            "case",
                            &directory.join("checkpoint.json"),
                            &source,
                            Some("deadline exceeded".into()),
                            elapsed,
                            &evidence
                        )
                        .unwrap()
                    );
                    keyringd_testkit::generate_html_report(report.path()).unwrap();
                    let html = std::fs::read_to_string(report.path().join("index.html")).unwrap();
                    assert!(html.contains("cases/0001/diagnostics/"));
                    assert!(html.contains("window.png"));
                    assert!(html.contains("ax-tree.txt"));
                    assert!(html.contains("deadline exceeded"));
                }
            }
        }
    }
}
