//! A userspace USB/IP HID backend for FIDO clients.
//!
//! Entry point: [`UsbIpHidBackend::new`], implemented in `usb_ip_hid_backend.rs`.
//! Inject the resulting backend into `FidoClient`. [`UsbIpHidPacketIo`] is its
//! associated I/O type; callers normally obtain it through the backend.
//!
//! The entry point composes HID packet I/O and USB/IP device enumeration.
//! Implementation dependencies: HID → connection → protocol. These modules
//! do not depend on the server manager or HTTP client.
mod usb_ip_error;
mod usb_ip_hid_backend;

mod connection;
mod hid;
mod protocol;

pub use hid::UsbIpHidPacketIo;
pub use usb_ip_error::UsbIpError;
pub use usb_ip_hid_backend::UsbIpHidBackend;

#[cfg(test)]
mod test_support;
