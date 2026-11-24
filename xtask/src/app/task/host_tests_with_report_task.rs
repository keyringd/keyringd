use crate::core::{model::Workspace, service};
use std::ffi::OsString;

use clap::Args;

use super::linux_system_test_task::LinuxSystemTestTask;

#[derive(Args, Clone, Debug)]
pub(crate) struct HostTestsWithReportTask {
    /// `--distribution` and `--package` apply to the Linux system tests only.
    #[command(flatten)]
    pub(crate) system: LinuxSystemTestTask,

    /// Arguments passed to `cargo test`; write them after `--`.
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) cargo_test_arguments: Vec<OsString>,
}

impl HostTestsWithReportTask {
    pub(crate) fn exec(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::test_workflow::host_tests_with_report(
            workspace,
            self.system.into(),
            &self.cargo_test_arguments,
        )
    }
}
