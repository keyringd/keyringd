use super::WindowsUsbIpClient;
use std::io;
use std::time::{Duration, Instant};

/// Owns one imported port. Does not detach unrelated USB/IP devices.
pub struct WindowsUsbIpAttachment {
    client: WindowsUsbIpClient,
    port: Option<u16>,
    serial: String,
    hid_instance: Option<String>,
}
impl WindowsUsbIpAttachment {
    pub(crate) fn new(client: WindowsUsbIpClient, port: u16, serial: String) -> Self {
        Self {
            client,
            port: Some(port),
            serial,
            hid_instance: None,
        }
    }
    pub fn port(&self) -> Option<u16> {
        self.port
    }
    pub fn hid_instance(&self) -> Option<&str> {
        self.hid_instance.as_deref()
    }
    /// Finds the test server's default VID/PID, then verifies the controller and port.
    /// usbip-win2's CLI serial is metadata; it is not a USB iSerialNumber.
    pub fn wait_ready(&mut self, timeout: Duration) -> io::Result<()> {
        let port = self.port.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotConnected,
                "USB/IP attachment is already detached",
            )
        })?;
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(id) = super::device_properties::ready_hid(port)? {
                self.hid_instance = Some(id);
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!(
                        "no healthy test-server HID on USB/IP port {:?} (serial {})",
                        self.port, self.serial
                    ),
                ));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    pub fn detach(&mut self) -> io::Result<()> {
        if let Some(port) = self.port {
            self.client
                .command(&["detach", "--port", &port.to_string()])?;
            self.port = None;
            if let Some(instance) = self.hid_instance.take() {
                let deadline = Instant::now() + Duration::from_secs(15);
                loop {
                    if !super::device_properties::present(&instance)? {
                        break;
                    }
                    if Instant::now() >= deadline {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            format!("USB/IP HID was not removed: {instance}"),
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
        Ok(())
    }
}
impl Drop for WindowsUsbIpAttachment {
    fn drop(&mut self) {
        if let Err(error) = self.detach() {
            eprintln!("USB/IP cleanup failed: {error}");
        }
    }
}
