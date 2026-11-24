use crate::core::{model::Workspace, service};
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Clone, Debug)]
pub(crate) struct LinuxSystemTestTask {
    /// Linux distribution used by the system-container testbed.
    #[arg(long, default_value = "ubuntu", value_parser = ["ubuntu", "fedora"])]
    pub(crate) distribution: String,

    /// Tests this existing package instead of building one automatically.
    #[arg(long, value_name = "PATH")]
    pub(crate) package: Option<PathBuf>,

    /// Runs only cases whose names contain this text.
    #[arg(long, value_name = "TEXT")]
    pub(crate) filter: Option<String>,
}

use crate::core::model::SystemTestRequest;
impl From<LinuxSystemTestTask> for SystemTestRequest {
    fn from(options: LinuxSystemTestTask) -> Self {
        SystemTestRequest::Linux {
            distribution: options.distribution,
            package: options.package,
            filter: options.filter,
        }
    }
}

impl LinuxSystemTestTask {
    pub(crate) fn exec(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::system_tests::run(workspace, &self.into(), false)
    }

    pub(crate) fn exec_with_report(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::system_tests::run(workspace, &self.into(), true)
    }
}
