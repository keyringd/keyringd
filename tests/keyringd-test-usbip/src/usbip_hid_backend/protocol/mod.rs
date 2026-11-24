//! USB/IP management messages, device records, and transfer framing.
pub(super) mod management;
pub(super) mod transfer;
mod usb_ip_device_record;
pub(super) use usb_ip_device_record::UsbIpDeviceRecord;
