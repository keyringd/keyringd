use crate::usb::{HidPacketIo, UsbHidDeviceId, UsbHidDeviceInfo, UsbHidError};
use futures_util::future::BoxFuture;
/// FIDO HID device enumeration and the interface that opens packet I/O for the specified device.
/// The device ID is interpreted by the backend that is issued and uses a stable value during the connection. Enumeration alone does not occupy the connection.
/// I/O handles CTAPHID packets excluding report IDs. The client will build, initialize, and select CTAP channels.
/// Indicates that the I/O errors that have been confirmed to disconnect the open connection are reconnectable with `UsbHidError::disconnected`.
pub trait UsbHidBackend: Send + Sync {
    fn enumerate(&self) -> BoxFuture<'_, Result<Vec<UsbHidDeviceInfo>, UsbHidError>>;
    fn open<'a>(
        &'a self,
        id: &'a UsbHidDeviceId,
    ) -> BoxFuture<'a, Result<Box<dyn HidPacketIo<Error = UsbHidError>>, UsbHidError>>;
}
