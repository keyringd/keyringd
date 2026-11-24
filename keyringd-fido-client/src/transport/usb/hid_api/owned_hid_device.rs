use super::hid_api_runtime::HidApiRuntime;

/// Device I/O may run on blocking workers, but closing a device must be
/// serialized with hidapi initialization, enumeration and opening.
pub(super) struct OwnedHidDevice {
    device: Option<hidapi::HidDevice>,
    owner: &'static HidApiRuntime,
}

impl OwnedHidDevice {
    pub(super) fn new(device: hidapi::HidDevice, owner: &'static HidApiRuntime) -> Self {
        Self {
            device: Some(device),
            owner,
        }
    }

    pub(super) fn raw(&self) -> &hidapi::HidDevice {
        self.device
            .as_ref()
            .expect("a live HID device retains its native handle")
    }
}

impl Drop for OwnedHidDevice {
    fn drop(&mut self) {
        if let Some(device) = self.device.take() {
            self.owner.close(device);
        }
    }
}
