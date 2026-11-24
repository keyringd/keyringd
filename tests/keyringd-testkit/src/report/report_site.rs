use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, ensure};
use maud::{DOCTYPE, html};

use crate::ReportOptions;
use crate::infrastructure::ResultStore;
use crate::model::Evidence;

/// Render the recorded results of this execution without consulting other hosts' sources.
pub fn generate_report_site(input: &Path, output: &Path, options: &ReportOptions) -> Result<()> {
    let input = fs::canonicalize(input)?;
    if output.exists() {
        let output = fs::canonicalize(output)?;
        ensure!(
            !input.starts_with(&output),
            "output must not contain the input results"
        );
        fs::remove_dir_all(output)?;
    }
    let mut reports = Vec::new();
    let mut distribution_groups = Vec::new();
    for category in ["ut", "system"] {
        let directory = input.join(category);
        if !directory.is_dir() {
            continue;
        }
        for os in directories(&directory)? {
            if category == "system"
                && os
                    .file_name()
                    .is_some_and(|name| name == "linux" || name == "wsl")
            {
                let distributions = directories(&os)?;
                reports.extend(
                    distributions
                        .iter()
                        .cloned()
                        .map(|directory| (directory, false)),
                );
                distribution_groups.push((os, distributions));
            } else {
                reports.push((os, category == "ut"));
            }
        }
    }
    for (group, distributions) in distribution_groups {
        let destination = output.join(group.strip_prefix(&input)?);
        fs::create_dir_all(&destination)?;
        let index = html! {
            (DOCTYPE)
            html lang="en" {
                head { meta charset="utf-8"; title { "Distribution test reports" } }
                body {
                    h1 { "Distribution test reports" }
                    ul {
                        @for distribution in distributions {
                            @let name = distribution.file_name().context("distribution directory has no name")?.to_string_lossy();
                            li { a href=(format!("{name}/index.html")) { (name) } }
                        }
                    }
                }
            }
        };
        fs::write(destination.join("index.html"), index.into_string())?;
    }
    ensure!(
        !reports.is_empty(),
        "no test result directories found in {}",
        input.display()
    );
    reports.sort();
    for (directory, is_unit_test_report) in &reports {
        let destination = output.join(directory.strip_prefix(&input)?);
        fs::create_dir_all(&destination)?;
        let metadata = directory.join("run-metadata.json");
        if metadata.is_file() {
            // Preserve observations from the original execution; rendering on
            // another host must not replace them with the renderer's tools.
            fs::copy(&metadata, destination.join("run-metadata.json"))?;
        }
        let results = ResultStore::for_report_directory(directory).load_recorded()?;
        for result in &results {
            for evidence in result
                .evidence
                .iter()
                .chain(result.steps.iter().flat_map(|step| &step.evidence))
            {
                if let Evidence::File { path, .. } | Evidence::Image { path, .. } = evidence {
                    let path = PathBuf::from(path.replace('\\', "/"));
                    ensure!(
                        path.components()
                            .all(|component| matches!(component, Component::Normal(_))),
                        "evidence must be relative to the results directory: {}",
                        path.display()
                    );
                    let target = destination.join(&path);
                    fs::create_dir_all(target.parent().context("evidence has no parent")?)?;
                    fs::copy(directory.join(&path), &target)
                        .with_context(|| format!("copy evidence {}", path.display()))?;
                }
            }
        }
        let project_root = crate::infrastructure::ProjectRootLocator::locate(&results);
        let requirements = project_root
            .as_ref()
            .map(|root| root.join("docs/internal/requirements.yaml"))
            .filter(|path| options.include_requirements && *is_unit_test_report && path.is_file());
        let html = super::render_html_report(
            results,
            &destination,
            project_root.as_deref(),
            options.source_base_url.as_deref(),
            requirements.is_some(),
        );
        if let Some(requirements) = requirements {
            crate::requirements_report::generate(&crate::requirements_report::ReportPaths {
                requirements,
                test_results: directory.clone(),
                output: destination.join("requirements.html"),
            })?;
        }
        fs::write(destination.join("index.html"), html)?;
    }
    let index = html! {
        (DOCTYPE)
        html lang="en" {
            head { meta charset="utf-8"; title { "Test reports" } }
            body {
                h1 { "Test reports" }
                ul {
                    @for (directory, _) in &reports {
                        @let path = directory.strip_prefix(&input)?.to_string_lossy().replace('\\', "/");
                        li { a href=(format!("{path}/index.html")) { (path) } }
                    }
                }
            }
        }
    };
    fs::write(output.join("index.html"), index.into_string())?;
    Ok(())
}

fn directories(path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            paths.push(entry.path());
        }
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(input: &Path, path: &str, source_file: &str) {
        let directory = input.join(path);
        fs::create_dir_all(directory.join("screenshots")).unwrap();
        fs::write(directory.join("screenshots/window.png"), b"evidence").unwrap();
        let result = serde_json::json!({
            "id": "case", "title": "Recorded case", "source_code": "captured source", "source_error": null,
            "crate_name": "keyringd-example", "module_path": "tests", "group": null,
            "source": {"file": source_file, "line": 1, "column": 1},
            "outcome": "failed", "duration_ms": 1, "failure": "test failed", "skipped_reason": null,
            "steps": [{
                "kind": "then", "description": "Recorded step",
                "source": {"file": source_file.replace('/', "\\"), "line": 2, "column": 1},
                "outcome": "failed", "duration_ms": 1
            }], "evidence": [{"kind":"image","name":"Window","path":"screenshots/window.png"}]
        });
        fs::write(directory.join("case.json"), result.to_string()).unwrap();
    }

    keyringd_testkit::scenario! {
        group "generate_report_site" {
            case "Generates current execution layout and evidence without source validation" {
                verify "render the current report and attachments without re-reading case sources" {
                    std::thread::spawn(|| {
                        let temporary = tempfile::tempdir().unwrap();
                        let input = temporary.path().join("results");
                        let output = temporary.path().join("reports");
                        record(
                            &input,
                            "ut/windows",
                            "tests/keyringd-example/tests/case.rs",
                        );
                        let metadata = crate::runner::ReportRunMetadata::capture(&input);
                        metadata.write(&input.join("ut/windows")).unwrap();
                        record(
                            &input,
                            "system/linux/ubuntu",
                            "keyringd-example/src/case.rs",
                        );
                        for os in ["macos", "windows"] {
                            record(
                                &input,
                                &format!("system/{os}"),
                                "keyringd-example/src/case.rs",
                            );
                        }
                        for distribution in ["ubuntu", "fedora"] {
                            record(
                                &input,
                                &format!("system/wsl/{distribution}"),
                                "keyringd-example/src/case.rs",
                            );
                        }
                        fs::create_dir_all(output.join("ut/macos")).unwrap();
                        fs::write(output.join("ut/macos/index.html"), "previous run").unwrap();
                        generate_report_site(
                            &input,
                            &output,
                            &ReportOptions {
                                source_base_url: Some("https://example.test/blob/revision".to_owned()),
                                ..Default::default()
                            },
                        )
                        .unwrap();
                        let index = fs::read_to_string(output.join("index.html")).unwrap();
                        assert!(index.contains("ut/windows/index.html"));
                        assert!(index.contains("system/linux/ubuntu/index.html"));
                        let wsl_index = fs::read_to_string(output.join("system/wsl/index.html")).unwrap();
                        for distribution in ["ubuntu", "fedora"] {
                            assert!(index.contains(&format!("system/wsl/{distribution}/index.html")));
                            assert!(wsl_index.contains(&format!("{distribution}/index.html")));
                            let report =
                                fs::read_to_string(output.join(format!("system/wsl/{distribution}/index.html")))
                                    .unwrap();
                            assert!(report.contains("Recorded case"));
                            assert_eq!(
                                fs::read(output.join(format!("system/wsl/{distribution}/screenshots/window.png")))
                                    .unwrap(),
                                b"evidence"
                            );
                        }
                        assert!(!index.contains("ut/macos/index.html"));
                        assert!(!output.join("ut/macos").exists());
                        let report = fs::read_to_string(output.join("ut/windows/index.html")).unwrap();
                        assert!(report.contains("href=\"requirements.html\""));
                        assert!(output.join("ut/windows/requirements.html").is_file());
                        assert!(report.contains("Recorded case"));
                        assert!(report.contains(&metadata.run_id));
                        let copied = crate::runner::ReportRunMetadata::load(&output.join("ut/windows"))
                            .unwrap()
                            .unwrap();
                        assert_eq!(copied.run_id, metadata.run_id);
                        assert!(report.contains("captured source"));
                        assert!(report.contains(
                            "https://example.test/blob/revision/tests/keyringd-example/tests/case.rs#L1"
                        ));
                        assert!(report.contains(
                            "https://example.test/blob/revision/tests/keyringd-example/tests/case.rs#L2"
                        ));
                        let linux_report =
                            fs::read_to_string(output.join("system/linux/ubuntu/index.html")).unwrap();
                        for line in [1, 2] {
                            assert!(linux_report.contains(&format!(
                                "https://example.test/blob/revision/keyringd-example/src/case.rs#L{line}"
                            )));
                        }
                        assert_eq!(
                            fs::read(output.join("ut/windows/screenshots/window.png")).unwrap(),
                            b"evidence"
                        );
                        assert!(input.join("ut/windows/case.json").is_file());
                        for path in [
                            "system/linux/ubuntu",
                            "system/macos",
                            "system/windows",
                            "system/wsl/ubuntu",
                            "system/wsl/fedora",
                        ] {
                            let directory = output.join(path);
                            let report = fs::read_to_string(directory.join("index.html")).unwrap();
                            assert!(!report.contains("href=\"requirements.html\""), "{path}");
                            assert!(!directory.join("requirements.html").exists(), "{path}");
                        }
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "generate_report_site" {
            case "Invalid results fail without being deleted" {
                verify "return the invalid-result error and retain the input record on disk" {
                    std::thread::spawn(|| {
                        let temporary = tempfile::tempdir().unwrap();
                        let input = temporary.path().join("results");
                        let result = input.join("ut/linux/broken.json");
                        fs::create_dir_all(result.parent().unwrap()).unwrap();
                        fs::write(&result, "invalid JSON").unwrap();
                        assert!(
                            generate_report_site(
                                &input,
                                &temporary.path().join("reports"),
                                &ReportOptions::default()
                            )
                            .is_err()
                        );
                        assert!(result.is_file());
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
