/// The identification value of the USB device that is only interpreted by the backend that is issued.
/// Even if you repeat enumeration with the same backend, use the same value for the same connected device.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UsbHidDeviceId(Vec<u8>);
impl UsbHidDeviceId {
    pub fn new(value: impl Into<Vec<u8>>) -> Self {
        Self(value.into())
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
