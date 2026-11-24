use super::ClientDataProvider;

/// Execution inputs for one assertion operation. Non-WebAuthn callers may
/// explicitly disable user presence on the Internal route.
/// User verification is specified by the WebAuthn request options.
pub struct GetAssertionsContext<'a> {
    pub(super) client_data_provider: &'a dyn ClientDataProvider,
    pub(super) usb_device_id: Option<crate::usb::UsbHidDeviceId>,
    pub(super) user_presence: bool,
}

impl<'a> GetAssertionsContext<'a> {
    pub fn new(client_data_provider: &'a dyn ClientDataProvider) -> Self {
        Self {
            client_data_provider,
            usb_device_id: None,
            user_presence: true,
        }
    }

    /// Targets one USB HID device instead of selecting an authenticator.
    pub fn with_usb_device_id(mut self, id: Option<crate::usb::UsbHidDeviceId>) -> Self {
        self.usb_device_id = id;
        self
    }

    /// Requires user presence by default. Only the Internal route supports false.
    pub fn with_user_presence(mut self, required: bool) -> Self {
        self.user_presence = required;
        self
    }
}
