//! Noise encryption and sending and receiving CTAP messages on established connections.
mod hybrid_ctap_transport;
mod hybrid_ctap_transport_error;
mod hybrid_data_channel;
mod hybrid_noise_error;
mod hybrid_noise_session;

pub use hybrid_ctap_transport::HybridCtapTransport;
pub use hybrid_ctap_transport_error::HybridCtapTransportError;
pub use hybrid_data_channel::HybridDataChannel;
pub use hybrid_noise_error::HybridNoiseError;
pub use hybrid_noise_session::HybridNoiseSession;

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
pub(crate) use hybrid_noise_session::strip_padding;

#[cfg(test)]
pub(crate) use hybrid_noise_session::PADDING_GRANULARITY;

#[cfg(test)]
pub(crate) use hybrid_noise_session::CTAP_MESSAGE_TYPE;

#[cfg(test)]
pub(crate) use hybrid_noise_session::pad_plaintext;
