use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};

use super::attached_usb_ip_port::AttachedUsbIpPort;
use super::usbip_executable::usbip_executable;
use crate::util::command;

pub(crate) struct UsbIpAttachment {
    port: AttachedUsbIpPort,
    hidraw: PathBuf,
}

impl UsbIpAttachment {
    pub(crate) fn attach(
        interrupted: &AtomicBool,
        host: &str,
        bus_id: &str,
        pending_removals: Arc<Mutex<Vec<PathBuf>>>,
    ) -> Result<Self> {
        AttachedUsbIpPort::wait_for_removals(&pending_removals)?;
        command::require_success(
            "vhci-hcd kernel module load",
            "modprobe",
            ["vhci-hcd"],
            Path::new("/"),
        )?;
        let status_path = Path::new("/sys/devices/platform/vhci_hcd.0/status");
        let before = attached_usbip_ports(status_path)?;
        let usbip = usbip_executable()?;
        command::require_success(
            "USB/IP authenticator attachment",
            &usbip,
            ["attach", "--remote", host, "--busid", bus_id],
            Path::new("/"),
        )?;

        let deadline = Instant::now() + Duration::from_secs(5);
        let port = loop {
            if let Some(port) = attached_usbip_ports(status_path)?
                .difference(&before)
                .next()
                .copied()
            {
                break port;
            }
            ensure!(
                Instant::now() < deadline,
                "unable to determine attached USB/IP port"
            );
            thread::sleep(Duration::from_millis(50));
        };

        let mut attached_port = AttachedUsbIpPort::new(port, usbip, pending_removals);
        let hidraw = wait_for_hidraw(interrupted, status_path, port, &mut attached_port)?;
        Ok(Self {
            port: attached_port,
            hidraw,
        })
    }

    pub(crate) fn hidraw(&self) -> &Path {
        &self.hidraw
    }

    pub(crate) fn allow_e2e_user(&self, uid: u32, gid: u32) -> Result<()> {
        std::os::unix::fs::chown(&self.hidraw, Some(uid), Some(gid))
            .context("failed to assign the owned test authenticator to the E2E user")?;
        fs::set_permissions(&self.hidraw, fs::Permissions::from_mode(0o600))
            .context("failed to restrict the owned test authenticator to the E2E user")
    }

    pub(crate) fn detach(&mut self) -> Result<()> {
        self.port.detach()
    }
}

fn attached_usbip_ports(path: &Path) -> Result<HashSet<u32>> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read USB/IP status {}", path.display()))?;
    Ok(contents
        .lines()
        .skip(1)
        .filter_map(|line| {
            let columns = line.split_whitespace().collect::<Vec<_>>();
            (columns.len() > 2 && columns[2] != "004")
                .then(|| columns[1].parse::<u32>().ok())
                .flatten()
        })
        .collect())
}

fn wait_for_hidraw(
    interrupted: &AtomicBool,
    status_path: &Path,
    port: u32,
    attached_port: &mut AttachedUsbIpPort,
) -> Result<PathBuf> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if interrupted.load(Ordering::SeqCst) {
            bail!("interrupted while waiting for the authenticator hidraw device");
        }
        if let Some(device) = usb_device_for_port(status_path, port)? {
            attached_port.remember_device(device.clone());
            if let Some(hidraw) = matching_hidraw(&device)? {
                return Ok(hidraw);
            }
        }
        ensure!(
            Instant::now() < deadline,
            "WebAuthn4J authenticator hidraw device did not appear for USB/IP port {port}"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

fn usb_device_for_port(status_path: &Path, port: u32) -> Result<Option<PathBuf>> {
    let status = fs::read_to_string(status_path)
        .with_context(|| format!("failed to read USB/IP status {}", status_path.display()))?;
    for line in status.lines().skip(1) {
        let columns = line.split_whitespace().collect::<Vec<_>>();
        if columns.len() >= 7
            && columns[1].parse::<u32>().ok() == Some(port)
            && columns[2] != "004"
            && columns[6] != "0-0"
        {
            return canonicalize_if_present(&Path::new("/sys/bus/usb/devices").join(columns[6]));
        }
    }
    Ok(None)
}

fn matching_hidraw(usb_device: &Path) -> Result<Option<PathBuf>> {
    for entry in fs::read_dir("/sys/class/hidraw")? {
        let entry = entry?;
        let Some(device) = canonicalize_if_present(&entry.path().join("device"))? else {
            continue;
        };
        // hidraw numbers are reused after detach. Match the attached USB device,
        // not a difference between snapshots of /dev names.
        if !device.starts_with(usb_device) {
            continue;
        }
        let uevent = match fs::read_to_string(device.join("uevent")) {
            Ok(uevent) => uevent,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error).context("failed to read HID device identity"),
        };
        let hidraw = Path::new("/dev").join(entry.file_name());
        if uevent
            .lines()
            .any(|line| line == "HID_ID=0003:00001234:0000F1D0")
            && hidraw.try_exists()?
        {
            return Ok(Some(hidraw));
        }
    }
    Ok(None)
}

fn canonicalize_if_present(path: &Path) -> Result<Option<PathBuf>> {
    match path.canonicalize() {
        Ok(path) => Ok(Some(path)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to resolve {}", path.display())),
    }
}
