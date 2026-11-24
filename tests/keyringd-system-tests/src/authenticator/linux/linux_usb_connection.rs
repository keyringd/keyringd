use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use crate::authenticator::UsbConnection;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

/// A reconnectable physical USB/IP lease; server credentials belong to the key.
pub(crate) struct LinuxUsbConnection {
    socket: PathBuf,
    id: String,
    stream: Option<BufReader<UnixStream>>,
    hidraw: Option<PathBuf>,
}

impl LinuxUsbConnection {
    pub(crate) fn from_environment(id: &str) -> Result<Self> {
        let socket = std::env::var_os("KEYRINGD_E2E_USB_CONTROL")
            .context("KEYRINGD_E2E_USB_CONTROL is required for a Linux USB security key")?;
        Ok(Self {
            socket: socket.into(),
            id: id.to_owned(),
            stream: None,
            hidraw: None,
        })
    }

    fn request(&mut self, op: &str) -> Result<Value> {
        if self.stream.is_none() {
            let stream = UnixStream::connect(&self.socket).with_context(|| {
                format!("connect to Linux USB/IP control {}", self.socket.display())
            })?;
            stream.set_read_timeout(Some(Duration::from_secs(30)))?;
            stream.set_write_timeout(Some(Duration::from_secs(5)))?;
            self.stream = Some(BufReader::new(stream));
        }
        let stream = self.stream.as_mut().unwrap();
        writeln!(stream.get_mut(), "{}", json!({"op": op, "id": self.id}))?;
        let mut response = String::new();
        ensure!(
            stream.read_line(&mut response)? > 0,
            "Linux USB/IP broker closed the fixture lease"
        );
        let response: Value = serde_json::from_str(&response)?;
        if let Some(error) = response.get("error") {
            anyhow::bail!("Linux USB/IP {op} failed: {error}");
        }
        response
            .get("ok")
            .cloned()
            .context("invalid Linux USB/IP broker response")
    }
}

impl UsbConnection for LinuxUsbConnection {
    fn connect(&mut self) -> Result<()> {
        if self.hidraw.is_none() {
            let response = self.request("attach")?;
            let path = response["hidraw"]
                .as_str()
                .context("USB/IP import did not return its hidraw path")?;
            self.hidraw = Some(PathBuf::from(path));
        }
        Ok(())
    }

    fn disconnect(&mut self) -> Result<()> {
        if self.hidraw.is_some() {
            self.request("detach")?;
            self.hidraw = None;
        }
        Ok(())
    }
}
