use crate::usbip_hid_backend::UsbIpError;
use std::net::SocketAddr;

pub(crate) struct UsbIpDeviceRecord {
    pub server: SocketAddr,
    pub bus_id: String,
    pub device_id: u32,
    pub vendor_id: u16,
    pub product_id: u16,
    pub interface_count: u8,
}
impl UsbIpDeviceRecord {
    pub(crate) fn parse(server: SocketAddr, bytes: &[u8; 312]) -> Result<Self, UsbIpError> {
        let bus_id = &bytes[256..288];
        let end = bus_id
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(UsbIpError::Protocol("unterminated bus ID"))?;
        let bus_id = std::str::from_utf8(&bus_id[..end])
            .map_err(|_| UsbIpError::Protocol("invalid bus ID"))?
            .to_owned();
        if bus_id.is_empty() {
            return Err(UsbIpError::Protocol("empty bus ID"));
        }
        let bus = u32::from_be_bytes(bytes[288..292].try_into().unwrap());
        let device = u32::from_be_bytes(bytes[292..296].try_into().unwrap());
        if bus > u16::MAX.into() || device > u16::MAX.into() {
            return Err(UsbIpError::Protocol("invalid device address"));
        }
        Ok(Self {
            server,
            bus_id,
            device_id: (bus << 16) | device,
            vendor_id: u16::from_be_bytes(bytes[300..302].try_into().unwrap()),
            product_id: u16::from_be_bytes(bytes[302..304].try_into().unwrap()),
            interface_count: bytes[311],
        })
    }
}
