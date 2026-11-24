use crate::core::{model::Workspace, service};
use clap::Args;

#[derive(Args, Clone, Debug)]
pub(crate) struct MacosSystemTestTask {
    /// Runs only UI case names containing this text.
    #[arg(long, value_name = "TEXT")]
    pub(crate) filter: Option<String>,
    /// Deadline for each scenario process; bounded diagnostic collection follows.
    #[arg(long, default_value_t = 180, value_parser = clap::value_parser!(u64).range(1..))]
    pub(crate) case_timeout_seconds: u64,
}

use crate::core::model::SystemTestRequest;
impl From<MacosSystemTestTask> for SystemTestRequest {
    fn from(options: MacosSystemTestTask) -> Self {
        SystemTestRequest::Macos {
            filter: options.filter,
            case_timeout_seconds: options.case_timeout_seconds,
        }
    }
}

impl MacosSystemTestTask {
    pub(crate) fn exec(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::system_tests::run(workspace, &self.into(), false)
    }

    pub(crate) fn exec_with_report(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::system_tests::run(workspace, &self.into(), true)
    }
}
