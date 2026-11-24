use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};

pub(super) struct AttachedUsbIpPort {
    port: Option<u32>,
    usbip: PathBuf,
    device: Option<PathBuf>,
    pending_removals: Arc<Mutex<Vec<PathBuf>>>,
}

impl AttachedUsbIpPort {
    pub(super) fn new(
        port: u32,
        usbip: PathBuf,
        pending_removals: Arc<Mutex<Vec<PathBuf>>>,
    ) -> Self {
        Self {
            port: Some(port),
            usbip,
            device: None,
            pending_removals,
        }
    }

    pub(super) fn remember_device(&mut self, path: PathBuf) {
        self.device = Some(path);
    }

    pub(super) fn detach(&mut self) -> Result<()> {
        if let Some(port) = self.port {
            let status = Command::new(&self.usbip)
                .args(["detach", "--port", &port.to_string()])
                .status()
                .with_context(|| format!("failed to detach USB/IP port {port}"))?;
            ensure!(
                status.success(),
                "USB/IP detach on port {port} failed with {status}"
            );
            self.port = None;
            if let Some(device) = self.device.take() {
                self.pending_removals.lock().unwrap().push(device);
            }
        }
        Self::wait_for_removals(&self.pending_removals)
    }

    // usbip detach can return before Linux removes the old device. Keep timed-out
    // paths in the broker so the next lease checks again before reusing a port.
    pub(super) fn wait_for_removals(pending: &Mutex<Vec<PathBuf>>) -> Result<()> {
        let mut pending = pending.lock().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while let Some(device) = pending.last() {
            if !device
                .try_exists()
                .with_context(|| format!("checking detached USB device {}", device.display()))?
            {
                pending.pop();
                continue;
            }
            ensure!(
                Instant::now() < deadline,
                "detached USB device still exists in sysfs: {}",
                device.display()
            );
            thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    }
}

impl Drop for AttachedUsbIpPort {
    fn drop(&mut self) {
        if self.port.is_some()
            && let Err(error) = self.detach()
        {
            eprintln!("USB/IP port cleanup: {error:#}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_timed_out_removal_blocks_port_reuse_until_the_old_device_disappears() {
        let sysfs = tempfile::tempdir().unwrap();
        let device = sysfs.path().join("1-1");
        std::fs::create_dir(&device).unwrap();
        let pending = Mutex::new(vec![device.clone()]);
        assert!(AttachedUsbIpPort::wait_for_removals(&pending).is_err());
        assert_eq!(
            pending.lock().unwrap().as_slice(),
            std::slice::from_ref(&device)
        );
        std::thread::scope(|scope| {
            scope.spawn(|| {
                thread::sleep(Duration::from_millis(100));
                std::fs::remove_dir(&device).unwrap();
            });
            AttachedUsbIpPort::wait_for_removals(&pending).unwrap();
            assert!(!device.exists());
        });
        assert!(pending.lock().unwrap().is_empty());
    }
}
