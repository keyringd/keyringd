use super::WindowsUsbIpAttachment;
use crate::environment::windows::command_output::run_command;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Uses an installed usbip-win2 client to import USB devices into Windows.
#[derive(Clone)]
pub struct WindowsUsbIpClient {
    executable: PathBuf,
}
impl WindowsUsbIpClient {
    /// Resolve the installed CLI using PATH or the installer's Program Files location.
    pub fn installed() -> io::Result<Self> {
        let program = "usbip.exe";
        for directory in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
            let executable = directory.join(program);
            if executable.is_file() {
                return Ok(Self { executable });
            }
        }
        if let Some(directory) = std::env::var_os("ProgramFiles") {
            let executable = PathBuf::from(directory).join("USBip").join(program);
            if executable.is_file() {
                return Ok(Self { executable });
            }
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "usbip-win2 is not installed: usbip.exe must be on PATH or in Program Files/USBip",
        ))
    }
    pub fn probe(&self) -> io::Result<String> {
        let version = self.command(&["--version"])?;
        self.command(&["port"])?;
        Ok(version.trim().to_owned())
    }
    pub fn attach(&self, server: SocketAddr, bus_id: &str) -> io::Result<WindowsUsbIpAttachment> {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        if sequence > u16::MAX as u64 {
            return Err(io::Error::other("USB/IP test serial sequence exhausted"));
        }
        let serial = format!("P4S{:08X}{:04X}", std::process::id(), sequence);
        let output = self.command(&[
            "--tcp-port",
            &server.port().to_string(),
            "attach",
            "--remote",
            &server.ip().to_string(),
            "--bus-id",
            bus_id,
            "--terse",
            "--once",
            "--serial",
            &serial,
        ])?;
        let port = output.trim().parse::<u16>().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("usbip-win2 returned no port: {output}"),
            )
        })?;
        let mut attachment = WindowsUsbIpAttachment::new(self.clone(), port, serial);
        attachment.wait_ready(Duration::from_secs(20))?;
        Ok(attachment)
    }
    pub(crate) fn command(&self, arguments: &[&str]) -> io::Result<String> {
        let mut command = Command::new(&self.executable);
        command.args(arguments);
        let output = run_command(command, Duration::from_secs(20))?;
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "usbip-win2 {arguments:?} failed ({}): {}{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}
