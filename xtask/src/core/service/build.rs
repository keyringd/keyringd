use crate::core::{
    driver::cargo,
    model::{HostPlatform, Workspace},
};
use anyhow::Result;

pub(crate) fn run(workspace: &Workspace, release: bool, locked: bool) -> Result<()> {
    let host = HostPlatform::parse(std::env::consts::OS)?;
    cargo::build(workspace, host.application_packages(), release, locked)
}
