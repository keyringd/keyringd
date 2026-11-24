//! Imported connections and concurrent transfer lifetimes, including cancellation.
mod usb_ip_connection;
mod usb_ip_pending_transfer;
mod usb_ip_transfer_guard;
mod usb_ip_transfer_result;
pub(super) use usb_ip_connection::UsbIpConnection;
