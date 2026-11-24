use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::util::command;

pub(crate) fn usbip_executable() -> Result<PathBuf> {
    let kernel = command::output("uname", ["-r"], Path::new("/"))?;
    let kernel_tool = Path::new("/usr/lib/linux-tools")
        .join(kernel.trim())
        .join("usbip");
    if kernel_tool.is_file() {
        return Ok(kernel_tool);
    }

    let mut installed = fs::read_dir("/usr/lib")
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok().map(|entry| entry.path().join("usbip")))
        .filter(|path| {
            path.parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("linux-tools-"))
                && path.is_file()
        })
        .collect::<Vec<_>>();
    installed.sort();
    installed.pop().ok_or_else(|| {
        anyhow::anyhow!("unable to locate a usbip executable under /usr/lib/linux-tools")
    })
}
