mod mock_ctap_transport;
pub(crate) use mock_ctap_transport::{MockCtapTransport, model_transport};

mod mock_hid_packet_io;
pub(crate) use mock_hid_packet_io::MockHidPacketIo;

mod mock_os_usb_backend;
pub(crate) use mock_os_usb_backend::MockOsUsbBackend;
mod mock_hybrid_channel_connector;
pub(crate) use mock_hybrid_channel_connector::{
    MockHybridChannelConnector, pending_hybrid_connector,
};

mod mock_usb_hid_backend;
pub(crate) use mock_usb_hid_backend::{MockUsbHidBackend, empty_usb_backend};
mod mock_fido_interaction_handler;
pub(crate) use mock_fido_interaction_handler::{MockFidoInteractionHandler, usb_only_handler};

#[cfg(test)]
mod assert_fido_error;
#[cfg(test)]
pub(crate) use assert_fido_error::assert_fido_error;
