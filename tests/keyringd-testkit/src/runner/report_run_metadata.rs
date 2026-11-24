use super::{ToolExecutionMetadata, command_observation::CommandObservation};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportRunMetadata {
    pub run_id: String,
    pub started_at_utc: Result<String, String>,
    pub operating_system: String,
    pub os_version: Result<String, String>,
    pub test_environment: Option<Result<String, String>>,
    pub source_revision: Result<String, String>,
    pub source_dirty: Result<bool, String>,
    pub tools: Vec<ToolExecutionMetadata>,
}

impl ReportRunMetadata {
    pub fn capture(workspace: &Path) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        #[cfg(windows)]
        let (started_at_utc, os_version) = (
            Ok(super::windows_run_information::utc_time()),
            super::windows_run_information::os_version(),
        );
        #[cfg(not(windows))]
        let (started_at_utc, os_version) = {
            (
                CommandObservation::run(
                    Path::new("date"),
                    &["-u", "+%Y-%m-%dT%H:%M:%SZ"],
                    workspace,
                ),
                if cfg!(target_os = "macos") {
                    CommandObservation::run(Path::new("sw_vers"), &[], workspace)
                } else {
                    CommandObservation::run(Path::new("uname"), &["-srv"], workspace)
                },
            )
        };
        Self {
            run_id: format!("{stamp}-{}", std::process::id()),
            started_at_utc,
            operating_system: std::env::consts::OS.into(),
            os_version,
            test_environment: None,
            source_revision: CommandObservation::run(
                Path::new("git"),
                &["rev-parse", "HEAD"],
                workspace,
            ),
            source_dirty: CommandObservation::run(
                Path::new("git"),
                &["status", "--porcelain", "--untracked-files=normal"],
                workspace,
            )
            .map(|output| !output.is_empty()),
            tools: Vec::new(),
        }
    }

    pub fn write(&self, report: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(report)?;
        std::fs::write(
            report.join("run-metadata.json"),
            serde_json::to_vec_pretty(self)?,
        )?;
        Ok(())
    }

    /// Runners add the executable they actually selected, including JAVA_HOME
    /// resolution and explicit SSH locations. Do not infer tool identity later.
    pub fn record_tool(report: &Path, tool: ToolExecutionMetadata) -> anyhow::Result<()> {
        let path = report.join("run-metadata.json");
        if !path.is_file() {
            return Ok(());
        }
        let mut run: Self = serde_json::from_slice(&std::fs::read(&path)?)?;
        run.tools.retain(|previous| previous.name != tool.name);
        run.tools.push(tool);
        run.write(report)
    }

    pub fn record_test_environment(
        report: &Path,
        environment: Result<String, String>,
    ) -> anyhow::Result<()> {
        let path = report.join("run-metadata.json");
        if !path.is_file() {
            return Ok(());
        }
        let mut run: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        run.test_environment = Some(environment);
        run.write(report)
    }

    pub fn load(report: &Path) -> anyhow::Result<Option<Self>> {
        let path = report.join("run-metadata.json");
        if !path.is_file() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_slice(&std::fs::read(path)?)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "ReportRunMetadata::load" {
            case "Selected executable replaces earlier observation and survives reload" {
                verify "persist the selected executable path in place of the earlier tool observation" {
                    std::thread::spawn(|| {
                        let directory = tempfile::tempdir().unwrap();
                        let run = ReportRunMetadata::capture(directory.path());
                        assert!(
                            run.source_revision.is_err(),
                            "no Git repository exists here"
                        );
                        let id = run.run_id.clone();
                        run.write(directory.path()).unwrap();
                        for path in ["/old/sshd", "/selected/sshd"] {
                            ReportRunMetadata::record_tool(
                                directory.path(),
                                ToolExecutionMetadata {
                                    name: "SSH server".into(),
                                    path: Some(path.into()),
                                    version: Ok("test version".into()),
                                },
                            )
                            .unwrap();
                        }
                        let restored = ReportRunMetadata::load(directory.path()).unwrap().unwrap();
                        crate::generate_html_report_with_options(
                            directory.path(),
                            &crate::ReportOptions {
                                include_requirements: false,
                                ..Default::default()
                            },
                        )
                        .unwrap();
                        assert!(directory.path().join("run-metadata.json").is_file());
                        assert!(
                            std::fs::read_to_string(directory.path().join("index.html"))
                                .unwrap()
                                .contains(&id)
                        );
                        let results =
                            super::super::ScenarioResults::load(directory.path(), "example::cases::").unwrap();
                        assert_eq!(results.to_string(), "0 passed, 0 skipped, 0 failed");
                        assert_eq!(restored.run_id, id);
                        assert_eq!(restored.tools.len(), 1);
                        assert_eq!(
                            restored.tools[0].path.as_deref(),
                            Some(Path::new("/selected/sshd"))
                        );
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
