use super::usbip::{WindowsUsbIpAttachment, WindowsUsbIpClient};
use crate::authenticator::UsbConnection;
use anyhow::Result;
use std::net::SocketAddr;

pub(crate) struct WindowsUsbConnection {
    server: SocketAddr,
    bus_id: String,
    client: WindowsUsbIpClient,
    attachment: Option<WindowsUsbIpAttachment>,
}
impl WindowsUsbConnection {
    pub(crate) fn new(server: SocketAddr, bus_id: String) -> Result<Self> {
        Ok(Self {
            server,
            bus_id,
            client: WindowsUsbIpClient::installed()?,
            attachment: None,
        })
    }
}
impl UsbConnection for WindowsUsbConnection {
    fn connect(&mut self) -> Result<()> {
        if self.attachment.is_none() {
            let attachment = self.client.attach(self.server, &self.bus_id)?;
            eprintln!(
                "Imported Security Key {} into port {:?}, HID {:?}",
                self.bus_id,
                attachment.port(),
                attachment.hid_instance()
            );
            self.attachment = Some(attachment);
        }
        Ok(())
    }
    fn disconnect(&mut self) -> Result<()> {
        if let Some(attachment) = self.attachment.as_mut() {
            attachment.detach()?;
        }
        self.attachment = None;
        Ok(())
    }
}
