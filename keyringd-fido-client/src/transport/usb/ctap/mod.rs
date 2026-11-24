//! CTAPHID initialization on HID packet I/O, message communication, cancellation and termination.
mod ctap_hid_transport;
mod ctap_hid_transport_error;

pub use ctap_hid_transport::CtapHidTransport;
pub use ctap_hid_transport_error::CtapHidTransportError;

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
pub(crate) use ctap_hid_transport::BROADCAST_CHANNEL;

#[cfg(test)]
pub(crate) use ctap_hid_transport::CHANNEL_BUSY_ERROR;

#[cfg(test)]
pub(crate) use ctap_hid_transport::CHANNEL_BUSY_RETRY_DELAY;

#[cfg(test)]
pub(crate) use ctap_hid_transport::MAX_CHANNEL_BUSY_RETRIES;
