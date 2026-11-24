use super::Error;
// Finalize a case even when its process exits before the scenario framework.
use crate::infrastructure::ResultStore;
use crate::model::{CaseOutcome, CaseResult, Evidence, SourceLocation};
use std::path::{Path, PathBuf};

/// The supervisor may only replace the result of the exact case it launched.
/// Returns whether the case passed, including the external process outcome.
pub fn finish_case(
    report: &Path,
    id: &str,
    checkpoint: &Path,
    fallback_source: &Path,
    failure: Option<String>,
    duration_ms: u128,
    artifacts: &[PathBuf],
) -> Result<bool, Error> {
    let store = ResultStore::for_report_directory(report);
    let mut candidates: Vec<_> = store
        .load_current()?
        .into_iter()
        .filter(|r| r.id == id)
        .collect();
    if candidates.len() > 1 {
        return Err("duplicate case results".into());
    }
    let missing_result = candidates.is_empty();
    let mut result = if let Some(result) = candidates.pop() {
        result
    } else {
        let checkpoint_result = std::fs::read(checkpoint)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<CaseResult>(&bytes).ok())
            .filter(|r| r.id == id);
        checkpoint_result.unwrap_or_else(|| CaseResult {
            id: id.into(),
            title: id.into(),
            rationale: None,
            specifications: vec![],
            source_code: None,
            source_error: Some("process exited before recording case metadata".into()),
            crate_name: id.split("::").next().unwrap_or(id).into(),
            module_path: id.split_once("::").map_or("", |(_, module)| module).into(),
            group: None,
            source: SourceLocation::new(fallback_source.to_string_lossy(), 1, 1),
            outcome: CaseOutcome::Failed,
            duration_ms,
            failure: Some("case result was not written".into()),
            skipped_reason: None,
            steps: vec![],
            evidence: vec![],
        })
    };
    if missing_result {
        result.outcome = CaseOutcome::Failed;
        result
            .failure
            .get_or_insert_with(|| "case result was not written".into());
        result.skipped_reason = None;
    }
    if let Some(failure) = failure {
        result.outcome = CaseOutcome::Failed;
        result.failure = Some(match result.failure {
            Some(previous) => format!("{previous}\n{failure}"),
            None => failure,
        });
        result.skipped_reason = None;
    }
    // Missing results and SKIP cannot be counted as success.
    if result.outcome == CaseOutcome::Skipped {
        result.outcome = CaseOutcome::Failed;
        result.failure = Some(format!(
            "selected UI case was skipped: {}",
            result.skipped_reason.take().unwrap_or_default()
        ));
    }
    result.duration_ms = duration_ms;
    for path in artifacts {
        let relative = path.strip_prefix(report)?;
        let name = relative.display().to_string();
        let path = name.clone();
        result
            .evidence
            .push(if relative.extension().is_some_and(|e| e == "png") {
                Evidence::Image { name, path }
            } else {
                Evidence::File { name, path }
            });
    }
    let passed = result.outcome == CaseOutcome::Passed;
    store.write(&result)?;
    Ok(passed)
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "finish_case" {
            case "A process without a result is a reported failure with artifacts" {
                verify "publish a failure with available evidence when the child exits without a result" {
                    std::thread::spawn(|| {
                        let root = tempfile::tempdir().unwrap();
                        let source = root.path().join("source.rs");
                        std::fs::write(&source, "test source").unwrap();
                        let log = root.path().join("scenario.log");
                        std::fs::write(&log, "hung here").unwrap();
                        assert!(
                            !finish_case(
                                root.path(),
                                "case",
                                &root.path().join("missing"),
                                &source,
                                Some("deadline exceeded".into()),
                                100,
                                &[log]
                            )
                            .unwrap()
                        );
                        crate::generate_html_report(root.path()).unwrap();
                        let html = std::fs::read_to_string(root.path().join("index.html")).unwrap();
                        assert!(html.contains("1 cases · 0 passed · 1 failed"));
                        assert!(html.contains("deadline exceeded"));
                        assert!(html.contains("href=\"scenario.log\""));
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "finish_case" {
            case "A saved pass is overridden by external failure and skip is rejected" {
                verify "replace a recorded pass after external failure and reject a skipped outcome" {
                    std::thread::spawn(|| {
                        let root = tempfile::tempdir().unwrap();
                        let source = root.path().join("source.rs");
                        std::fs::write(&source, "source").unwrap();
                        let checkpoint = root.path().join("checkpoint.json");
                        finish_case(root.path(), "case", &checkpoint, &source, None, 1, &[]).unwrap();
                        let store = ResultStore::for_report_directory(root.path());
                        let mut result = store.load_current().unwrap().pop().unwrap();
                        result.outcome = CaseOutcome::Passed;
                        result.failure = None;
                        store.write(&result).unwrap();
                        assert!(
                            !finish_case(
                                root.path(),
                                "case",
                                &checkpoint,
                                &source,
                                Some("process timed out after recording PASS".into()),
                                2,
                                &[]
                            )
                            .unwrap()
                        );
                        assert_eq!(
                            store.load_current().unwrap()[0].outcome,
                            CaseOutcome::Failed
                        );
                        result.outcome = CaseOutcome::Skipped;
                        result.skipped_reason = Some("missing dependency".into());
                        store.write(&result).unwrap();
                        assert!(!finish_case(root.path(), "case", &checkpoint, &source, None, 3, &[]).unwrap());
                        assert!(
                            store.load_current().unwrap()[0]
                                .failure
                                .as_ref()
                                .unwrap()
                                .contains("skipped")
                        );
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
