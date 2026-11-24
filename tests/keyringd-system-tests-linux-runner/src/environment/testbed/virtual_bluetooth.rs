use std::collections::BTreeSet;
use std::fs::{self, File};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};

/// BlueZ sees only the PC controller. The Phone connects directly to btvirt's
/// HCI server and shares its radio without becoming a kernel/BlueZ adapter.
pub(super) struct VirtualBluetooth {
    child: Child,
    pub(super) scanner: String,
    pub(super) advertiser_address: SocketAddr,
}

impl VirtualBluetooth {
    pub(super) fn start(runtime: &Path) -> Result<Option<Self>> {
        if !Path::new("/dev/vhci").exists() {
            return Ok(None);
        }
        let before = controllers()?;
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let advertiser_address = listener.local_addr()?;
        drop(listener);
        let log = File::create(runtime.join("btvirt.log"))?;
        let stderr = log.try_clone()?;
        let child = Command::new("btvirt")
            .args(["-L", "-l1", &format!("-t{}", advertiser_address.port())])
            .stdout(log)
            .stderr(stderr)
            .spawn()
            .context("start btvirt with a PC controller and Phone HCI server")?;
        let mut virtual_bluetooth = Self {
            child,
            scanner: String::new(),
            advertiser_address,
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = virtual_bluetooth.child.try_wait()? {
                bail!("btvirt exited before Bluetooth was ready: {status}");
            }
            let added: Vec<_> = controllers()?.difference(&before).cloned().collect();
            if added.len() == 1
                && TcpStream::connect_timeout(&advertiser_address, Duration::from_millis(100))
                    .is_ok()
            {
                virtual_bluetooth.scanner = added[0].clone();
                return Ok(Some(virtual_bluetooth));
            }
            ensure!(
                Instant::now() < deadline,
                "btvirt PC controller or Phone HCI server is not ready (found {added:?}, endpoint {advertiser_address})"
            );
            thread::sleep(Duration::from_millis(100));
        }
    }
}

pub(super) fn controllers() -> Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for entry in fs::read_dir("/sys/class/bluetooth")? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name.starts_with("hci") {
            names.insert(name);
        }
    }
    Ok(names)
}

impl Drop for VirtualBluetooth {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
