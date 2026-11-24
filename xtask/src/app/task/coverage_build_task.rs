use crate::core::{model::Workspace, service};
use clap::Args;
use std::ffi::OsString;

#[derive(Args)]
pub(crate) struct CoverageBuildTask {
    /// Arguments passed to cargo build, after `--`.
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) arguments: Vec<OsString>,
}

impl CoverageBuildTask {
    pub(crate) fn exec(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::coverage::build(workspace, &self.arguments)
    }
}
