use crate::core::{driver::workspace, model::Workspace};
use anyhow::Result;
use std::path::PathBuf;

pub(crate) fn load(root: Option<PathBuf>) -> Result<Workspace> {
    Ok(Workspace::new(workspace::resolve_root(root)?))
}
