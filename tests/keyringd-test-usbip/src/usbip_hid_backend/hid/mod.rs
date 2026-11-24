//! HID descriptor interpretation and packet I/O over an imported connection.
mod report_descriptor;
mod usb_ip_hid_interface;
mod usb_ip_hid_packet_io;
pub use usb_ip_hid_packet_io::UsbIpHidPacketIo;
