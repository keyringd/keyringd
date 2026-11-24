use std::path::PathBuf;
/// Execution configuration, independent of the CLI that invokes the runner.
#[derive(Clone, Debug)]
pub struct RunnerOptions {
    pub workspace_root: PathBuf,
    pub report_directory: PathBuf,
    pub filter: Option<String>,
    pub case_timeout_seconds: u64,
}
