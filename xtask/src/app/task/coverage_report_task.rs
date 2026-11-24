use crate::core::{model::Workspace, service};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub(crate) struct CoverageReportTask {
    pub(crate) input: PathBuf,
    pub(crate) output: PathBuf,
    /// Required suite/OS pairs; missing results make aggregation fail.
    #[arg(long = "require")]
    pub(crate) required: Vec<String>,
}

impl CoverageReportTask {
    pub(crate) fn exec(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::coverage::merge(workspace, &self.input, &self.output, &self.required)
    }
}
