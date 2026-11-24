use super::usb_ip_transfer_result::UsbIpTransferResult;
use crate::usbip_hid_backend::UsbIpError;
use tokio::sync::oneshot;
pub(crate) struct UsbIpPendingTransfer {
    pub input: bool,
    pub maximum: usize,
    pub response: Option<oneshot::Sender<Result<UsbIpTransferResult, UsbIpError>>>,
}
