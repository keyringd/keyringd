use super::operation_context::OperationContext;
use crate::TransportSelection;
use crate::client::PreparedMakeCredential;

pub(crate) struct MakeCredentialOperationContext<'a> {
    pub(crate) operation: OperationContext<'a>,
    pub(crate) prepared: &'a PreparedMakeCredential,
    pub(crate) usb_device_id: Option<crate::usb::UsbHidDeviceId>,
    pub(crate) transport_selection: Option<TransportSelection>,
}

impl<'a> MakeCredentialOperationContext<'a> {
    pub(crate) fn new(
        prepared: &'a PreparedMakeCredential,
        operation: OperationContext<'a>,
        transport_selection: Option<TransportSelection>,
        usb_device_id: Option<crate::usb::UsbHidDeviceId>,
    ) -> Self {
        Self {
            operation,
            prepared,
            transport_selection: if usb_device_id.is_some() {
                Some(TransportSelection::Usb)
            } else {
                transport_selection
            },
            usb_device_id,
        }
    }
}
