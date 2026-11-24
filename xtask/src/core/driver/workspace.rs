use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

pub(in crate::core) fn resolve_root(root: Option<PathBuf>) -> Result<PathBuf> {
    let root = match root {
        Some(root) => root,
        None => Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .context("xtask must be located directly below the workspace root")?
            .to_owned(),
    };
    let root = std::path::absolute(root).context("cannot resolve workspace directory")?;
    ensure!(
        root.join("Cargo.toml").is_file(),
        "workspace Cargo.toml is missing"
    );
    Ok(root)
}
