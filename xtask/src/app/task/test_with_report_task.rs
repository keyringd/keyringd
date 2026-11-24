use crate::core::{model::Workspace, service};
use std::ffi::OsString;

use clap::Args;

#[derive(Args, Clone, Debug, Default)]
pub(crate) struct TestWithReportTask {
    /// Arguments passed to `cargo test`; write them after `--`.
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) cargo_test_arguments: Vec<OsString>,
}

impl TestWithReportTask {
    pub(crate) fn exec(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::test_workflow::standard_with_report(workspace, &self.cargo_test_arguments)
    }
}
