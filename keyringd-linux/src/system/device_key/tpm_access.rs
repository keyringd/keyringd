use std::io;
use std::path::Path;

pub(super) const RESOURCE_MANAGER: &str = "/dev/tpmrm0";
const GRANT_HELPER: &str = "/usr/libexec/keyringd/grant-tpm-access";
const PKEXEC: &str = "/usr/bin/pkexec";

pub(super) fn check_resource_manager_access() -> io::Result<()> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(RESOURCE_MANAGER)
        .map(drop)
}

pub(crate) fn can_grant_tpm_access() -> bool {
    Path::new(PKEXEC).is_file() && Path::new(GRANT_HELPER).is_file()
}

/// Returns false when the user dismisses the administrator authentication dialog.
pub(crate) async fn grant_tpm_access() -> io::Result<bool> {
    let output = tokio::process::Command::new(PKEXEC)
        .args(["--disable-internal-agent", GRANT_HELPER])
        .stdin(std::process::Stdio::null())
        .output()
        .await?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(126) => Ok(false),
        _ => Err(io::Error::other(format!(
            "TPM access helper exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim(),
        ))),
    }
}
