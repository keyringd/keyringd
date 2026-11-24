use anyhow::{Result, ensure};

use super::Environment;

pub(super) const ADAPTER_VISIBILITY_HELPER: &str =
    "/usr/local/libexec/keyringd-e2e-bluetooth-adapters";

/// Restores the container's HCI directory even if an availability case panics.
pub struct HiddenBluetoothAdapters {
    pub(super) restored: bool,
}

impl HiddenBluetoothAdapters {
    pub async fn restore(mut self, environment: &Environment) -> Result<()> {
        let output = environment
            .run("sudo", &["-n", ADAPTER_VISIBILITY_HELPER, "show"])
            .await?;
        ensure!(
            output.status.success(),
            "restoring adapters failed: {}",
            output.stderr
        );
        self.restored = true;
        Ok(())
    }
}

impl Drop for HiddenBluetoothAdapters {
    fn drop(&mut self) {
        if !self.restored {
            let result = std::process::Command::new("timeout")
                .args([
                    "-k",
                    "2s",
                    "5s",
                    "sudo",
                    "-n",
                    ADAPTER_VISIBILITY_HELPER,
                    "show",
                ])
                .output();
            tracing::trace!(?result, "Bluetooth adapter visibility cleanup completed");
        }
    }
}
