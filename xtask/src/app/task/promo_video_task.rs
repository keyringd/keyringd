use crate::core::{model::Workspace, service::documentation};
use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum PromoVideoTask {
    /// Renders, verifies, and publishes the 60-second introduction video.
    Build,
}

impl PromoVideoTask {
    pub(crate) fn exec(self, workspace: &Workspace) -> anyhow::Result<()> {
        match self {
            Self::Build => documentation::build_promo_video(workspace),
        }
    }
}
