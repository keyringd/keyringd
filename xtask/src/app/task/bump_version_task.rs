use crate::core::{model::Workspace, service};
use clap::Args;

#[derive(Args)]
pub(crate) struct BumpVersionTask;

impl BumpVersionTask {
    pub(crate) fn exec(self, workspace: &Workspace) -> anyhow::Result<()> {
        println!("{}", service::version::bump(workspace)?);
        Ok(())
    }
}
