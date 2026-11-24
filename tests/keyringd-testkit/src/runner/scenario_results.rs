use super::ScenarioSelection;
use crate::model::{CaseOutcome, CaseResult};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    path::Path,
};

/// Typed report results for one scenario executable, independent of other suites.
pub struct ScenarioResults {
    results: BTreeMap<String, CaseResult>,
}
impl ScenarioResults {
    /// `id_prefix` is the crate and test executable prefix, ending in `::`.
    /// Unlike report rendering, verification never deletes malformed or stale results.
    pub fn load(report_directory: &Path, id_prefix: &str) -> Result<Self> {
        ensure!(
            id_prefix.ends_with("::") && id_prefix.len() > 2,
            "scenario result prefix must end in ::"
        );
        let directory = report_directory.to_path_buf();
        let mut results = BTreeMap::new();
        if !directory.exists() {
            return Ok(Self { results });
        }
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            if !crate::infrastructure::ResultStore::is_result_file(&path) {
                continue;
            }
            let value: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)
                .with_context(|| format!("invalid scenario result JSON: {}", path.display()))?;
            let id = value["id"].as_str().context("scenario result has no id")?;
            let Some(case) = id.strip_prefix(id_prefix) else {
                continue;
            };
            let case = case.to_owned();
            let result: CaseResult = serde_json::from_value(value)
                .with_context(|| format!("invalid scenario result: {}", path.display()))?;
            ensure!(
                results.insert(case.clone(), result).is_none(),
                "duplicate case result: {case}"
            );
        }
        Ok(Self { results })
    }
    /// Skipped cases count as recorded results; missing, unexpected and failed cases are errors.
    pub fn verify(&self, selection: &ScenarioSelection) -> Result<()> {
        let actual: BTreeSet<_> = self.results.keys().cloned().collect();
        let expected = selection.cases();
        ensure!(
            &actual == expected,
            "case results do not match selected tests; missing: {:?}, unexpected: {:?}",
            expected.difference(&actual).collect::<Vec<_>>(),
            actual.difference(expected).collect::<Vec<_>>()
        );
        let failed: Vec<_> = self
            .results
            .iter()
            .filter(|(_, result)| result.outcome == CaseOutcome::Failed)
            .map(|(case, _)| case.as_str())
            .collect();
        ensure!(
            failed.is_empty(),
            "scenario cases failed: {}",
            failed.join(", ")
        );
        Ok(())
    }
    /// Whether the selected case has written a result, including a recorded skip.
    pub fn contains(&self, case: &str) -> bool {
        self.results.contains_key(case)
    }

    pub fn passed(&self) -> usize {
        self.count(CaseOutcome::Passed)
    }
    pub fn failed(&self) -> usize {
        self.count(CaseOutcome::Failed)
    }
    pub fn skipped(&self) -> usize {
        self.count(CaseOutcome::Skipped)
    }
    fn count(&self, outcome: CaseOutcome) -> usize {
        self.results
            .values()
            .filter(|result| result.outcome == outcome)
            .count()
    }
}
impl fmt::Display for ScenarioResults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} passed, {} skipped, {} failed",
            self.passed(),
            self.skipped(),
            self.failed()
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn write(directory: &Path, filename: &str, case: &str, outcome: &str) {
        fs::create_dir_all(directory).unwrap();
        let result = serde_json::json!({
            "id":format!("suite::scenarios::{case}"), "title":case, "source_code":null, "source_error":null,
            "crate_name":"suite", "module_path":"scenarios", "source":{"file":"test.rs","line":1,"column":1},
            "outcome":outcome, "duration_ms":0, "failure":null, "skipped_reason":null, "steps":[], "evidence":[]
        });
        fs::write(directory.to_path_buf().join(filename), result.to_string()).unwrap();
    }
    keyringd_testkit::scenario! {
        group "ScenarioResults::load" {
            case "Verifies passed and skipped cases without counting other suites" {
                verify "count only selected case results and accept both passed and skipped outcomes" {
                    std::thread::spawn(|| {
                        let directory = tempfile::tempdir().unwrap();
                        write(directory.path(), "pass.json", "pass", "passed");
                        write(directory.path(), "skip.json", "skip", "skipped");
                        fs::write(
                            directory.path().join("other.json"),
                            r#"{"id":"other::suite::case"}"#,
                        )
                        .unwrap();
                        let results = ScenarioResults::load(directory.path(), "suite::scenarios::").unwrap();
                        results
                            .verify(&ScenarioSelection::from_listing("pass: test\nskip: test").unwrap())
                            .unwrap();
                        assert_eq!(results.to_string(), "1 passed, 1 skipped, 0 failed");
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "ScenarioResults::load" {
            case "Rejects missing unexpected duplicate failed and malformed results" {
                verify "fail suite verification for incorrect case membership, duplicate or unreadable records, and failures" {
                    std::thread::spawn(|| {
                        let directory = tempfile::tempdir().unwrap();
                        let selected = ScenarioSelection::from_listing("chosen: test").unwrap();
                        assert!(
                            ScenarioResults::load(directory.path(), "suite::scenarios::")
                                .unwrap()
                                .verify(&selected)
                                .is_err()
                        );
                        write(directory.path(), "case.json", "unexpected", "passed");
                        assert!(
                            ScenarioResults::load(directory.path(), "suite::scenarios::")
                                .unwrap()
                                .verify(&selected)
                                .is_err()
                        );
                        write(directory.path(), "case.json", "chosen", "failed");
                        assert!(
                            ScenarioResults::load(directory.path(), "suite::scenarios::")
                                .unwrap()
                                .verify(&selected)
                                .is_err()
                        );
                        write(directory.path(), "duplicate.json", "chosen", "passed");
                        assert!(ScenarioResults::load(directory.path(), "suite::scenarios::").is_err());
                        fs::remove_file(directory.path().join("duplicate.json")).unwrap();
                        fs::write(
                            directory.path().join("case.json"),
                            r#"{"id":"suite::scenarios::chosen","outcome":"unknown"}"#,
                        )
                        .unwrap();
                        assert!(ScenarioResults::load(directory.path(), "suite::scenarios::").is_err());
                        assert!(directory.path().join("case.json").exists());
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
