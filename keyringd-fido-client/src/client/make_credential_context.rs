use super::ClientDataProvider;

/// Execution inputs for one registration. User presence is always required.
/// User verification is specified by the WebAuthn creation options.
pub struct MakeCredentialContext<'a> {
    pub(super) client_data_provider: &'a dyn ClientDataProvider,
    pub(super) usb_device_id: Option<crate::usb::UsbHidDeviceId>,
}

impl<'a> MakeCredentialContext<'a> {
    pub fn new(client_data_provider: &'a dyn ClientDataProvider) -> Self {
        Self {
            client_data_provider,
            usb_device_id: None,
        }
    }

    /// Targets one USB HID device instead of selecting an authenticator.
    pub fn with_usb_device_id(mut self, id: Option<crate::usb::UsbHidDeviceId>) -> Self {
        self.usb_device_id = id;
        self
    }
}
