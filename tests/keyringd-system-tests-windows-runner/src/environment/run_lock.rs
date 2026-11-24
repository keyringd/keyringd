use anyhow::{Context, Result};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};
pub(crate) struct RunLock {
    _file: File,
}
impl RunLock {
    pub fn acquire(cache: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(cache.join("run.lock"))?;
        file.try_lock()
            .context("another Windows-host system test run is active")?;
        Ok(Self { _file: file })
    }
}
