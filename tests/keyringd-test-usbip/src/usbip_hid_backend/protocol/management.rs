use super::usb_ip_device_record::UsbIpDeviceRecord;
use crate::usbip_hid_backend::UsbIpError;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub(crate) const VERSION: u16 = 0x0111;
pub(crate) fn management_header(command: u16) -> [u8; 8] {
    let mut b = [0; 8];
    b[..2].copy_from_slice(&VERSION.to_be_bytes());
    b[2..4].copy_from_slice(&command.to_be_bytes());
    b
}
pub(crate) async fn response(stream: &mut TcpStream, command: u16) -> Result<(), UsbIpError> {
    let mut b = [0; 8];
    stream.read_exact(&mut b).await?;
    if u16::from_be_bytes(b[..2].try_into().unwrap()) != VERSION
        || u16::from_be_bytes(b[2..4].try_into().unwrap()) != command
    {
        return Err(UsbIpError::Protocol("management response header"));
    }
    let status = i32::from_be_bytes(b[4..].try_into().unwrap());
    if status != 0 {
        return Err(UsbIpError::Status(status));
    }
    Ok(())
}
pub(crate) async fn enumerate(server: SocketAddr) -> Result<Vec<UsbIpDeviceRecord>, UsbIpError> {
    let mut stream = TcpStream::connect(server).await?;
    stream.write_all(&management_header(0x8005)).await?;
    response(&mut stream, 0x0005).await?;
    let count = stream.read_u32().await?;
    if count > 256 {
        return Err(UsbIpError::Protocol("too many exported devices"));
    }
    let mut devices = Vec::new();
    for _ in 0..count {
        let mut b = [0; 312];
        stream.read_exact(&mut b).await?;
        let device = UsbIpDeviceRecord::parse(server, &b)?;
        let mut interfaces = vec![0; usize::from(device.interface_count) * 4];
        stream.read_exact(&mut interfaces).await?;
        if interfaces.as_chunks::<4>().0.iter().any(|i| i[0] == 3) {
            devices.push(device);
        }
    }
    Ok(devices)
}
