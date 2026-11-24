use crate::core::{model::Workspace, service};
use clap::Args;

#[derive(Args, Clone, Debug)]
pub(crate) struct WindowsSystemTestTask {
    #[arg(long)]
    pub(crate) filter: Option<String>,
}

use crate::core::model::SystemTestRequest;
impl From<WindowsSystemTestTask> for SystemTestRequest {
    fn from(options: WindowsSystemTestTask) -> Self {
        SystemTestRequest::Windows {
            filter: options.filter,
        }
    }
}

impl WindowsSystemTestTask {
    pub(crate) fn exec(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::system_tests::run(workspace, &self.into(), false)
    }

    pub(crate) fn exec_with_report(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::system_tests::run(workspace, &self.into(), true)
    }
}
