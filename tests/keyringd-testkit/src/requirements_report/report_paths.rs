use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportPaths {
    pub requirements: PathBuf,
    pub test_results: PathBuf,
    pub output: PathBuf,
}
