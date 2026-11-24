use anyhow::{Result, ensure};
use std::path::PathBuf;

use super::Environment;

/// Enables the PXP BLE advertisement relay extension only for its E2E cases.
pub struct RemoteHybridAdvertisements {
    pub(super) drop_in: PathBuf,
    pub(super) restored: bool,
}

impl RemoteHybridAdvertisements {
    pub async fn restore(mut self, environment: &Environment) -> Result<()> {
        environment.stop_installed_application().await?;
        self.restore_file()?;
        let output = environment.systemctl_user(&["daemon-reload"]).await?;
        ensure!(
            output.status.success(),
            "systemd user reload failed: {}",
            output.stderr
        );
        self.restored = true;
        Ok(())
    }

    fn restore_file(&self) -> Result<()> {
        if self.drop_in.exists() {
            std::fs::remove_file(&self.drop_in)?;
        }
        Ok(())
    }
}

impl Drop for RemoteHybridAdvertisements {
    fn drop(&mut self) {
        if !self.restored {
            let _ = std::process::Command::new("systemctl")
                .args([
                    "--user",
                    "stop",
                    "keyringd-agent.socket",
                    "keyringd-desktop.service",
                ])
                .status();
            if self.restore_file().is_ok() {
                let _ = std::process::Command::new("systemctl")
                    .args(["--user", "daemon-reload"])
                    .status();
            }
        }
    }
}
