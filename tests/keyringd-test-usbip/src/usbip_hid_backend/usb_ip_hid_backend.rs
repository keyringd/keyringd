use super::hid::UsbIpHidPacketIo;
use crate::usbip_hid_backend::{UsbIpError, protocol::management};
use futures_util::future::BoxFuture;
use keyringd_fido_client::backend::UsbHidBackend;
use keyringd_fido_client::usb::{HidPacketIo, UsbHidError};
use keyringd_fido_client::usb::{UsbHidDeviceId, UsbHidDeviceInfo};
use std::net::SocketAddr;
use std::time::Duration;

/// Connects directly to USB/IP exports without installing a virtual USB controller.
#[derive(Clone, Debug)]
pub struct UsbIpHidBackend {
    servers: Vec<SocketAddr>,
}
impl UsbIpHidBackend {
    pub fn new(servers: impl IntoIterator<Item = SocketAddr>) -> Self {
        let mut servers = servers.into_iter().collect::<Vec<_>>();
        servers.sort_unstable();
        servers.dedup();
        Self { servers }
    }
}
impl UsbIpHidBackend {
    async fn enumerate(&self) -> Result<Vec<UsbHidDeviceInfo>, UsbIpError> {
        let mut result = Vec::new();
        for server in &self.servers {
            let devices =
                tokio::time::timeout(Duration::from_secs(5), management::enumerate(*server))
                    .await
                    .map_err(|_| UsbIpError::TimedOut)?;
            match devices {
                Ok(devices) => result.extend(devices.into_iter().map(|device| UsbHidDeviceInfo {
                    id: UsbHidDeviceId::new(
                        format!("{}#{}", device.server, device.bus_id).into_bytes(),
                    ),
                    vendor_id: device.vendor_id,
                    product_id: device.product_id,
                    manufacturer: None,
                    product: Some(format!("USB/IP {} {}", device.server, device.bus_id)),
                    serial_number: None,
                })),
                Err(UsbIpError::Io(error))
                    if error.kind() == std::io::ErrorKind::ConnectionRefused => {}
                Err(error) => return Err(error),
            }
        }
        Ok(result)
    }
    async fn open(&self, id: &UsbHidDeviceId) -> Result<UsbIpHidPacketIo, UsbIpError> {
        let identity =
            std::str::from_utf8(id.as_bytes()).map_err(|_| UsbIpError::DeviceNotFound)?;
        let (server, bus) = identity.split_once('#').ok_or(UsbIpError::DeviceNotFound)?;
        let server: SocketAddr = server.parse().map_err(|_| UsbIpError::DeviceNotFound)?;
        if !self.servers.contains(&server) {
            return Err(UsbIpError::DeviceNotFound);
        }
        tokio::time::timeout(Duration::from_secs(10), async {
            let device = management::enumerate(server)
                .await?
                .into_iter()
                .find(|d| d.bus_id == bus)
                .ok_or(UsbIpError::DeviceNotFound)?;
            UsbIpHidPacketIo::open(&device).await
        })
        .await
        .map_err(|_| UsbIpError::TimedOut)?
    }
}
impl UsbHidBackend for UsbIpHidBackend {
    fn enumerate(&self) -> BoxFuture<'_, Result<Vec<UsbHidDeviceInfo>, UsbHidError>> {
        Box::pin(async move { self.enumerate().await.map_err(UsbHidError::new) })
    }
    fn open<'a>(
        &'a self,
        id: &'a UsbHidDeviceId,
    ) -> BoxFuture<'a, Result<Box<dyn HidPacketIo<Error = UsbHidError>>, UsbHidError>> {
        Box::pin(async move {
            let io = self.open(id).await.map_err(UsbHidError::new)?;
            Ok(Box::new(io) as Box<dyn HidPacketIo<Error = UsbHidError>>)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usbip_hid_backend::protocol::management::management_header;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    keyringd_testkit::scenario! {
        group "UsbIpHidBackend::enumerate" {
            case "Devlist enumerates every hid export on one server and consumes all interfaces" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "list every exported HID device and consume the complete interface list" {
                    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                    let address = listener.local_addr().unwrap();
                    let serve = async {
                        let (mut stream, _) = listener.accept().await.unwrap();
                        let mut request = [0; 8];
                        stream.read_exact(&mut request).await.unwrap();
                        assert_eq!(request, management_header(0x8005));
                        let mut response = management_header(0x0005).to_vec();
                        response.extend_from_slice(&3u32.to_be_bytes());
                        for (index, interfaces) in [
                            (1, vec![[8, 0, 0, 0], [3, 0, 0, 0]]),
                            (2, vec![[8, 0, 0, 0]]),
                            (3, vec![[3, 0, 0, 0]]),
                        ] {
                            let mut record = [0; 312];
                            let bus = format!("1-{index}");
                            record[256..256 + bus.len()].copy_from_slice(bus.as_bytes());
                            record[288..292].copy_from_slice(&1u32.to_be_bytes());
                            record[292..296].copy_from_slice(&(index as u32).to_be_bytes());
                            record[311] = interfaces.len() as u8;
                            response.extend_from_slice(&record);
                            for interface in interfaces {
                                response.extend_from_slice(&interface);
                            }
                        }
                        for fragment in response.chunks(7) {
                            stream.write_all(fragment).await.unwrap();
                        }
                    };
                    let backend = UsbIpHidBackend::new([address, address]);
                    let (devices, ()) = tokio::join!(backend.enumerate(), serve);
                    let devices = devices.unwrap();
                    assert_eq!(devices.len(), 2);
                    assert_eq!(
                        devices[0].id.as_bytes(),
                        format!("{address}#1-1").as_bytes()
                    );
                    assert_eq!(
                        devices[1].id.as_bytes(),
                        format!("{address}#1-3").as_bytes()
                    );
                }
            }
        }
    }
}
