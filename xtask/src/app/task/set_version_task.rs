use crate::core::{model::Workspace, service};
use clap::Args;

#[derive(Args)]
pub(crate) struct SetVersionTask {
    version: String,
}

impl SetVersionTask {
    pub(crate) fn exec(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::version::set(workspace, &self.version)
    }
}
