use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};

/// Captures only the shared Authenticator server output produced by one case.
pub(super) struct AuthenticatorLogCapture {
    path: PathBuf,
    start: u64,
}

impl AuthenticatorLogCapture {
    pub(super) fn start(directory: &Path) -> Result<Self> {
        let path = directory.join("authenticator.log");
        let start = fs::metadata(&path)?.len();
        Ok(Self { path, start })
    }

    pub(super) fn attach_to_failed_case(self, directory: &Path, id: &str) -> Result<()> {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let mut result: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
            if result["id"] != id {
                continue;
            }
            if result["outcome"] != "failed" {
                return Ok(());
            }
            let mut log = File::open(&self.path)?;
            let end = log.metadata()?.len();
            ensure!(
                end >= self.start,
                "Authenticator log was truncated during {id}"
            );
            log.seek(SeekFrom::Start(self.start))?;
            let mut content = Vec::new();
            log.take(end - self.start).read_to_end(&mut content)?;
            result["evidence"]
                .as_array_mut()
                .context("case result must contain an evidence array")?
                .push(serde_json::json!({
                    "kind": "log",
                    "name": "Authenticator log",
                    "content": String::from_utf8(content).context("Authenticator log must be UTF-8")?
                }));
            fs::write(&path, serde_json::to_vec_pretty(&result)?)?;
            return Ok(());
        }
        anyhow::bail!("failed scenario did not record a result for {id}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn record(directory: &Path, outcome: &str) -> serde_json::Value {
        let result = serde_json::json!({
            "id": "keyringd-system-tests::scenarios::case",
            "title": "Recorded case", "crate_name": "keyringd-system-tests",
            "module_path": "scenarios", "group": null,
            "source": {"file": "tests/keyringd-system-tests/tests/main.rs", "line": 1, "column": 1},
            "source_code": "captured source", "source_error": null,
            "outcome": outcome, "duration_ms": 1,
            "failure": if outcome == "failed" { Some("case failed") } else { None },
            "skipped_reason": null, "steps": [],
            "evidence": [{"kind":"text", "name":"Existing evidence", "content":"preserved"}]
        });
        fs::write(
            directory.join("case.json"),
            serde_json::to_vec(&result).unwrap(),
        )
        .unwrap();
        result
    }

    keyringd_testkit::scenario! {
        group "AuthenticatorLogCapture::start" {
            case "Failed case keeps its log slice in the published report" {
                verify "attach the authenticator log slice to the failed case's published result" {
                    let temporary = tempfile::tempdir().unwrap();
                    let results = temporary.path().join("results/system/windows");
                    fs::create_dir_all(&results).unwrap();
                    let log = results.join("authenticator.log");
                    fs::write(&log, "previous case log\n").unwrap();
                    let capture = AuthenticatorLogCapture::start(&results).unwrap();
                    writeln!(
                        File::options().append(true).open(&log).unwrap(),
                        "Automatically approving get_assertion consent request"
                    )
                    .unwrap();
                    let original = record(&results, "failed");
                    capture
                        .attach_to_failed_case(&results, "keyringd-system-tests::scenarios::case")
                        .unwrap();
                    writeln!(
                        File::options().append(true).open(&log).unwrap(),
                        "next case log"
                    )
                    .unwrap();

                    let result: serde_json::Value =
                        serde_json::from_slice(&fs::read(results.join("case.json")).unwrap()).unwrap();
                    assert_eq!(result["failure"], original["failure"]);
                    assert_eq!(result["evidence"][0], original["evidence"][0]);
                    assert_eq!(
                        result["evidence"][1]["content"],
                        "Automatically approving get_assertion consent request\n"
                    );

                    let published = temporary.path().join("published");
                    keyringd_testkit::generate_report_site(
                        &temporary.path().join("results"),
                        &published,
                        &keyringd_testkit::ReportOptions {
                            include_requirements: false,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                    fs::remove_file(log).unwrap();
                    let html = fs::read_to_string(published.join("system/windows/index.html")).unwrap();
                    assert!(html.contains("Authenticator log"));
                    assert!(html.contains("Automatically approving get_assertion consent request"));
                    assert!(!html.contains("previous case log"));
                    assert!(!html.contains("next case log"));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AuthenticatorLogCapture::start" {
            case "Successful and skipped cases do not receive authenticator logs" {
                verify "omit authenticator log attachments for passed and skipped outcomes" {
                    for outcome in ["passed", "skipped"] {
                        let directory = tempfile::tempdir().unwrap();
                        fs::write(directory.path().join("authenticator.log"), "server log").unwrap();
                        let capture = AuthenticatorLogCapture::start(directory.path()).unwrap();
                        let original = record(directory.path(), outcome);
                        capture
                            .attach_to_failed_case(
                                directory.path(),
                                "keyringd-system-tests::scenarios::case",
                            )
                            .unwrap();
                        let result: serde_json::Value =
                            serde_json::from_slice(&fs::read(directory.path().join("case.json")).unwrap()).unwrap();
                        assert_eq!(result, original);
                    }
                }
            }
        }
    }
}
