//! USB common device information and input/output interfaces. Each sub-module is in charge of communication, report analysis, and actual machine implementation.
#![cfg_attr(test, allow(dead_code))]

pub(crate) mod ctap;
#[cfg(any(test, target_os = "linux", target_os = "macos"))]
pub(crate) mod hid_api;
mod hid_packet_io;
pub(crate) mod report;
mod usb_hid_device_id;
mod usb_hid_device_info;
mod usb_hid_error;

pub use ctap::{CtapHidTransport, CtapHidTransportError};
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use hid_api::{HidApiBackend, HidApiIoError, HidApiPacketIo};
pub use hid_packet_io::HidPacketIo;
pub use report::{HidReportDescriptorError, HidReportLayout, parse_report_layout};
pub use usb_hid_device_id::UsbHidDeviceId;
pub use usb_hid_device_info::UsbHidDeviceInfo;
pub use usb_hid_error::UsbHidError;
