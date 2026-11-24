use super::{report_descriptor::is_fido_report, usb_ip_hid_interface::UsbIpHidInterface};
use crate::usbip_hid_backend::UsbIpError;
use crate::usbip_hid_backend::{connection::UsbIpConnection, protocol::UsbIpDeviceRecord};
use futures_util::future::BoxFuture;
use keyringd_fido_client::usb::UsbHidError;
use keyringd_fido_client::usb::{HidPacketIo, HidReportLayout, parse_report_layout};

/// Owns one imported USB/IP connection. Dropping it closes the connection.
pub struct UsbIpHidPacketIo {
    connection: UsbIpConnection,
    interface: UsbIpHidInterface,
    layout: HidReportLayout,
}
impl UsbIpHidPacketIo {
    pub(crate) async fn open(device: &UsbIpDeviceRecord) -> Result<Self, UsbIpError> {
        let connection = UsbIpConnection::import(device).await?;
        let header = connection.descriptor(2, 0, 9).await?;
        if header.len() != 9 || header[1] != 2 {
            return Err(UsbIpError::Protocol("configuration descriptor header"));
        }
        let length = u16::from_le_bytes([header[2], header[3]]);
        if !(9..=4096).contains(&length) {
            return Err(UsbIpError::Protocol("configuration descriptor too large"));
        }
        let configuration = connection.descriptor(2, 0, length).await?;
        let interface = UsbIpHidInterface::parse(&configuration)?;
        connection.configure(interface.configuration).await?;
        if interface.report_length > 4096 {
            return Err(UsbIpError::Protocol("report descriptor too large"));
        }
        let report = connection
            .descriptor(0x22, u16::from(interface.number), interface.report_length)
            .await?;
        if report.len() != usize::from(interface.report_length) || !is_fido_report(&report) {
            return Err(UsbIpError::Protocol("not a FIDO HID interface"));
        }
        let layout = parse_report_layout(&report)?;
        let report_id = usize::from(layout.uses_report_ids());
        if layout.input_packet_size() + report_id > interface.input_size
            || layout.output_packet_size() + report_id > interface.output_size
        {
            return Err(UsbIpError::Protocol(
                "HID report exceeds interrupt endpoint packet size",
            ));
        }
        Ok(Self {
            connection,
            interface,
            layout,
        })
    }
}
impl HidPacketIo for UsbIpHidPacketIo {
    type Error = UsbHidError;
    fn input_packet_size(&self) -> usize {
        self.layout.input_packet_size()
    }
    fn output_packet_size(&self) -> usize {
        self.layout.output_packet_size()
    }
    fn write_packet<'a>(&'a mut self, packet: &'a [u8]) -> BoxFuture<'a, Result<(), UsbHidError>> {
        Box::pin(async move {
            self.write_packet(packet)
                .await
                .map_err(UsbIpError::into_hid_error)
        })
    }
    fn read_packet<'a>(&'a mut self) -> BoxFuture<'a, Result<Vec<u8>, UsbHidError>> {
        Box::pin(async move { self.read_packet().await.map_err(UsbIpError::into_hid_error) })
    }
}

impl UsbIpHidPacketIo {
    async fn write_packet(&mut self, packet: &[u8]) -> Result<(), UsbIpError> {
        if packet.len() != self.output_packet_size() {
            return Err(UsbIpError::Protocol("wrong HID output packet length"));
        }
        let mut report = Vec::new();
        if self.layout.uses_report_ids() {
            report.push(self.layout.report_id());
        }
        report.extend_from_slice(packet);
        let reply = self
            .connection
            .submit(
                false,
                self.interface.output_endpoint,
                report.len(),
                [0; 8],
                &report,
                self.interface.output_interval,
            )
            .await?;
        if reply.actual_length != report.len() {
            return Err(UsbIpError::Protocol("short HID output transfer"));
        }
        Ok(())
    }
    async fn read_packet(&mut self) -> Result<Vec<u8>, UsbIpError> {
        let length = self.input_packet_size() + usize::from(self.layout.uses_report_ids());
        let reply = self
            .connection
            .submit(
                true,
                self.interface.input_endpoint,
                length,
                [0; 8],
                &[],
                self.interface.input_interval,
            )
            .await?;
        if reply.data.len() != length {
            return Err(UsbIpError::Protocol("short HID input transfer"));
        }
        if self.layout.uses_report_ids() {
            if reply.data[0] != self.layout.report_id() {
                return Err(UsbIpError::Protocol("unexpected HID input report ID"));
            }
            Ok(reply.data[1..].to_vec())
        } else {
            Ok(reply.data)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usbip_hid_backend::test_support::{connect, reply, request};
    use tokio::io::AsyncReadExt;

    keyringd_testkit::scenario! {
        group "UsbIpHidPacketIo::write_packet" {
            case "Report ids are added to usb output and removed from usb input" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "insert the report ID for output and strip it from incoming data" {
                    let (connection, mut server) = connect().await;
                    let descriptor = [
                        0x06, 0xd0, 0xf1, 0x09, 0x01, 0xa1, 0x01, 0x85, 0x07, 0x75, 0x08, 0x95, 0x40, 0x09, 0x20,
                        0x81, 0x02, 0x09, 0x21, 0x91, 0x02, 0xc0,
                    ];
                    let mut io = UsbIpHidPacketIo {
                        connection,
                        interface: UsbIpHidInterface {
                            configuration: 1,
                            number: 0,
                            report_length: descriptor.len() as u16,
                            input_endpoint: 1,
                            output_endpoint: 2,
                            input_size: 65,
                            output_size: 65,
                            input_interval: 1,
                            output_interval: 1,
                        },
                        layout: parse_report_layout(&descriptor).unwrap(),
                    };
                    let packet = [0x42; 64];
                    let server_work = async {
                        let output = request(&mut server).await;
                        assert_eq!(u32::from_be_bytes(output[12..16].try_into().unwrap()), 0);
                        assert_eq!(u32::from_be_bytes(output[16..20].try_into().unwrap()), 2);
                        assert_eq!(u32::from_be_bytes(output[24..28].try_into().unwrap()), 65);
                        let mut report = [0; 65];
                        server.read_exact(&mut report).await.unwrap();
                        assert_eq!(report[0], 7);
                        assert_eq!(&report[1..], &packet);
                        let mut response = [0; 48];
                        response[..4].copy_from_slice(&3u32.to_be_bytes());
                        response[4..8].copy_from_slice(&output[4..8]);
                        response[24..28].copy_from_slice(&65u32.to_be_bytes());
                        use tokio::io::AsyncWriteExt;
                        server.write_all(&response).await.unwrap();
                        let input = request(&mut server).await;
                        assert_eq!(u32::from_be_bytes(input[12..16].try_into().unwrap()), 1);
                        assert_eq!(u32::from_be_bytes(input[16..20].try_into().unwrap()), 1);
                        reply(&mut server, &input, 3, &report, 0).await;
                    };
                    let client_work = async {
                        io.write_packet(&packet).await.unwrap();
                        assert_eq!(io.read_packet().await.unwrap(), packet);
                    };
                    tokio::join!(client_work, server_work);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UsbIpHidInterface::parse" {
            case "Malformed configuration descriptors are rejected without panicking" {
                verify "return errors for malformed interface descriptor data" {
                    let valid = [
                        9, 2, 41, 0, 1, 1, 0, 0x80, 50, 9, 4, 0, 0, 2, 3, 0, 0, 0, 9, 0x21, 0x11, 0x01, 0, 1, 0x22,
                        34, 0, 7, 5, 0x81, 3, 64, 0, 5, 7, 5, 0x02, 3, 64, 0, 5,
                    ];
                    assert!(UsbIpHidInterface::parse(&valid).is_ok());
                    for length in 0..valid.len() {
                        assert!(UsbIpHidInterface::parse(&valid[..length]).is_err());
                    }
                    let mut invalid = valid;
                    invalid[9] = 0;
                    assert!(UsbIpHidInterface::parse(&invalid).is_err());
                    invalid = valid;
                    invalid[9] = 255;
                    assert!(UsbIpHidInterface::parse(&invalid).is_err());
                }
            }
        }
    }
}
