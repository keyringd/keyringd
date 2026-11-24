use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest, Sha256};

use crate::infrastructure::{ReportInfrastructureError, SourceSnippetLoader};
use crate::model::CaseResult;

static TEMPORARY_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) struct ResultStore {
    results_directory: PathBuf,
}

impl ResultStore {
    pub(crate) fn is_result_file(path: &Path) -> bool {
        path.extension()
            .is_some_and(|extension| extension == "json")
            && path
                .file_name()
                .is_none_or(|name| name != "run-metadata.json")
    }

    pub(crate) fn for_test_execution() -> Self {
        let report_directory = std::env::var_os("KEYRINGD_TEST_REPORT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(Self::default_report_directory);
        Self::for_report_directory(report_directory)
    }

    pub(crate) fn default_report_directory() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("test support crate must be inside tests in the workspace")
            .join("target")
            .join("test-reports")
    }

    pub(crate) fn for_report_directory(report_directory: impl AsRef<Path>) -> Self {
        Self {
            results_directory: report_directory.as_ref().to_path_buf(),
        }
    }

    pub(crate) fn write(&self, result: &CaseResult) -> Result<(), ReportInfrastructureError> {
        fs::create_dir_all(&self.results_directory).map_err(|source| {
            ReportInfrastructureError::ResultIo {
                path: self.results_directory.clone(),
                source,
            }
        })?;
        let json = serde_json::to_vec_pretty(result).map_err(|source| {
            ReportInfrastructureError::ResultEncode {
                id: result.id.clone(),
                source,
            }
        })?;
        let destination = self
            .results_directory
            .join(format!("{}.json", result_file_name(&result.id)));
        let temporary = self.results_directory.join(format!(
            ".{}.{}.{}.tmp",
            result_file_name(&result.id),
            std::process::id(),
            TEMPORARY_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let write_outcome =
            write_new_file(&temporary, &json).and_then(|()| replace_file(&temporary, &destination));
        if write_outcome.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        write_outcome
    }

    pub(crate) fn load_recorded(&self) -> Result<Vec<CaseResult>, ReportInfrastructureError> {
        let mut results = Vec::new();
        for entry in fs::read_dir(&self.results_directory).map_err(|source| {
            ReportInfrastructureError::ResultIo {
                path: self.results_directory.clone(),
                source,
            }
        })? {
            let path = entry
                .map_err(|source| ReportInfrastructureError::ResultIo {
                    path: self.results_directory.clone(),
                    source,
                })?
                .path();
            if !Self::is_result_file(&path) {
                continue;
            }
            let json = fs::read(&path).map_err(|source| ReportInfrastructureError::ResultIo {
                path: path.clone(),
                source,
            })?;
            results.push(
                serde_json::from_slice(&json)
                    .map_err(|source| ReportInfrastructureError::ResultDecode { path, source })?,
            );
        }
        Ok(results)
    }

    pub(crate) fn load_current(&self) -> Result<Vec<CaseResult>, ReportInfrastructureError> {
        let mut results = Vec::new();
        if !self.results_directory.exists() {
            return Ok(results);
        }
        let entries = fs::read_dir(&self.results_directory).map_err(|source| {
            ReportInfrastructureError::ResultIo {
                path: self.results_directory.clone(),
                source,
            }
        })?;
        for entry in entries {
            let path = entry
                .map_err(|source| ReportInfrastructureError::ResultIo {
                    path: self.results_directory.clone(),
                    source,
                })?
                .path();
            if !Self::is_result_file(&path) {
                continue;
            }
            let json = fs::read(&path).map_err(|source| ReportInfrastructureError::ResultIo {
                path: path.clone(),
                source,
            })?;
            let Ok(result) = serde_json::from_slice::<CaseResult>(&json) else {
                remove_stale_result(&path)?;
                continue;
            };
            if result_is_current(&result) {
                results.push(result);
            } else {
                remove_stale_result(&path)?;
            }
        }
        Ok(results)
    }
}

fn remove_stale_result(path: &Path) -> Result<(), ReportInfrastructureError> {
    fs::remove_file(path).map_err(|source| ReportInfrastructureError::ResultIo {
        path: path.to_path_buf(),
        source,
    })
}

fn replace_file(temporary: &Path, destination: &Path) -> Result<(), ReportInfrastructureError> {
    if destination.exists() {
        fs::remove_file(destination).map_err(|source| ReportInfrastructureError::ResultIo {
            path: destination.to_path_buf(),
            source,
        })?;
    }
    fs::rename(temporary, destination).map_err(|source| ReportInfrastructureError::ResultIo {
        path: destination.to_path_buf(),
        source,
    })
}

fn result_is_current(result: &CaseResult) -> bool {
    let Some(source_code) = &result.source_code else {
        return SourceSnippetLoader::source_path(&result.source).is_ok_and(|path| path.is_file());
    };
    let line_count = source_code.lines().count();
    let Ok(line_count) = u32::try_from(line_count) else {
        return false;
    };
    let Some(end_line) = result.source.line.checked_add(line_count.saturating_sub(1)) else {
        return false;
    };
    SourceSnippetLoader::load(&result.source, end_line)
        .is_ok_and(|current_source| current_source == *source_code)
}

fn write_new_file(path: &Path, contents: &[u8]) -> Result<(), ReportInfrastructureError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|source| ReportInfrastructureError::ResultIo {
            path: path.to_path_buf(),
            source,
        })?;
    file.write_all(contents)
        .and_then(|()| file.sync_all())
        .map_err(|source| ReportInfrastructureError::ResultIo {
            path: path.to_path_buf(),
            source,
        })
}

fn result_file_name(id: &str) -> String {
    let readable_prefix: String = id
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .take(48)
        .collect();
    let digest = Sha256::digest(id.as_bytes());
    let digest = digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{readable_prefix}-{digest}")
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "result_file_name" {
            case "Distinguishes ids that have the same readable file name" {
                verify "generate distinct result filenames for IDs with the same readable stem" {
                    std::thread::spawn(|| {
                        assert_ne!(
                            result_file_name("crate::module::case"),
                            result_file_name("crate__module__case")
                        );
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "result_file_name" {
            case "Bounds the result file name length" {
                verify "keep the generated filename within the permitted length" {
                    std::thread::spawn(|| {
                        assert!(result_file_name(&"very-long-module::".repeat(100)).len() <= 81);
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ResultStore::write" {
            case "Removes a result that uses an obsolete or invalid schema" {
                verify "remove stale records whose JSON or schema version cannot be read" {
                    std::thread::spawn(|| {
                        let directory = tempfile::tempdir().unwrap();
                        let invalid_result = directory.path().join("invalid.json");
                        fs::write(&invalid_result, b"not json").unwrap();
                        let store = ResultStore {
                            results_directory: directory.path().to_path_buf(),
                        };

                        let results = store.load_current().unwrap();

                        assert!(results.is_empty());
                        assert!(!invalid_result.exists());
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ResultStore::write" {
            case "Replaces the previous result for the same case" {
                verify "overwrite the stored result for the identical case ID" {
                    std::thread::spawn(|| {
                        let directory = tempfile::tempdir().unwrap();
                        let store = ResultStore {
                            results_directory: directory.path().to_path_buf(),
                        };
                        let source_file = directory.path().join("case.rs");
                        fs::write(&source_file, "case source\n").unwrap();
                        let mut result = case_result(&source_file, "first");
                        store.write(&result).unwrap();
                        result.title = "second".to_owned();

                        store.write(&result).unwrap();

                        let loaded = store.load_current().unwrap();
                        assert_eq!(loaded.len(), 1);
                        assert_eq!(loaded[0].title, "second");
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ResultStore::write" {
            case "Removes a result whose case source has changed" {
                verify "discard the record when the saved source revision no longer matches" {
                    std::thread::spawn(|| {
                        let directory = tempfile::tempdir().unwrap();
                        let store = ResultStore {
                            results_directory: directory.path().to_path_buf(),
                        };
                        let source_file = directory.path().join("case.rs");
                        fs::write(&source_file, "case source\n").unwrap();
                        store.write(&case_result(&source_file, "case")).unwrap();
                        fs::write(&source_file, "changed source\n").unwrap();

                        assert!(store.load_current().unwrap().is_empty());
                        assert!(
                            !store
                                .results_directory
                                .join(format!("{}.json", result_file_name("crate::case")))
                                .exists()
                        );
                        assert!(source_file.is_file());
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    fn case_result(source_file: &Path, title: &str) -> CaseResult {
        use crate::model::{CaseOutcome, SourceLocation};

        CaseResult {
            id: "crate::case".to_owned(),
            title: title.to_owned(),
            rationale: None,
            specifications: Vec::new(),
            source_code: Some("case source".to_owned()),
            source_error: None,
            crate_name: "crate".to_owned(),
            module_path: "module".to_owned(),
            group: None,
            source: SourceLocation::new(source_file.to_string_lossy(), 1, 1),
            outcome: CaseOutcome::Passed,
            duration_ms: 0,
            failure: None,
            skipped_reason: None,
            steps: Vec::new(),
            evidence: Vec::new(),
        }
    }
}
