use crate::core::{model::Workspace, service};
use clap::Args;

#[derive(Args)]
pub(crate) struct BuildTask {
    /// Builds optimized application binaries.
    #[arg(long)]
    pub(crate) release: bool,

    /// Requires Cargo.lock to remain unchanged.
    #[arg(long)]
    pub(crate) locked: bool,
}

impl BuildTask {
    pub(crate) fn exec(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::build::run(workspace, self.release, self.locked)
    }
}
