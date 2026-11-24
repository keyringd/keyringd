use super::Environment;
use anyhow::{Result, ensure};

/// Restores the PC btvirt controller even if an availability scenario panics.
pub struct PoweredOffVirtualBluetooth {
    pub(super) controller: String,
    pub(super) restored: bool,
}

impl PoweredOffVirtualBluetooth {
    pub async fn restore(mut self, environment: &Environment) -> Result<()> {
        let controller = &self.controller;
        let output = environment
            .run_virtual_btmgmt(controller, "power on")
            .await?;
        ensure!(
            output.status.success(),
            "restoring {controller} failed: {}",
            output.stderr
        );
        self.restored = true;
        Ok(())
    }
}

impl Drop for PoweredOffVirtualBluetooth {
    fn drop(&mut self) {
        if self.restored {
            return;
        }
        let controller = &self.controller;
        let _ = std::process::Command::new("timeout")
            .args(["-k", "2s", "5s", "script", "-qefc"])
            .arg(format!("sudo -n btmgmt -i {controller} power on"))
            .arg("/dev/null")
            .status();
    }
}
