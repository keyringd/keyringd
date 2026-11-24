use std::path::PathBuf;

use crate::Distribution;

#[derive(Clone, Debug)]
pub struct TestbedExecutionRequest {
    pub distribution: Distribution,
    pub workspace: PathBuf,
    pub package: Option<PathBuf>,
    pub test_executable: PathBuf,
    pub report_directory: PathBuf,
    pub image: PathBuf,
    pub machine: String,
    pub owner_uid: u32,
    pub owner_gid: u32,
    pub authenticator_control_url: String,
    pub filter: Option<String>,
}

impl TestbedExecutionRequest {
    pub(crate) fn runtime_directory(&self) -> PathBuf {
        self.workspace.join(&self.report_directory).join("runtime")
    }
}
