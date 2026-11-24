//! Windows device imports through the installed usbip-win2 client.

mod device_info_set;
mod device_properties;
mod windows_usb_ip_attachment;
mod windows_usb_ip_client;

pub use windows_usb_ip_attachment::WindowsUsbIpAttachment;
pub use windows_usb_ip_client::WindowsUsbIpClient;
