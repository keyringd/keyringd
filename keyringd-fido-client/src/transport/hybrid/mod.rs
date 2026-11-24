//! Hybrid connection with QR and BLE ads. Connector builds from search to channel establishment.
pub(crate) mod ble;
pub(crate) mod crypto;
pub(crate) mod ctap;
mod hybrid_channel_connection;
mod hybrid_channel_connector;
mod hybrid_connection_error;
mod hybrid_qr_initiated_transaction;
mod hybrid_transport_unavailable_reason;
mod system_hybrid_channel_connector;
pub(crate) mod tunnel;

pub(crate) use ble::HybridBleScannerAvailability;
pub use ble::{HybridAdvertisementRelay, HybridBleError, HybridBleScanner};
pub use crypto::{HybridCryptoError, HybridSecret64};
pub use ctap::{
    HybridCtapTransport, HybridCtapTransportError, HybridDataChannel, HybridNoiseError,
    HybridNoiseSession,
};
pub(crate) use hybrid_channel_connection::HybridChannelConnection;
pub(crate) use hybrid_channel_connector::HybridChannelConnector;
pub(crate) use hybrid_connection_error::HybridConnectionError;
pub use hybrid_qr_initiated_transaction::HybridQrInitiatedTransaction;
pub use hybrid_transport_unavailable_reason::HybridTransportUnavailableReason;
pub use keyringd_fido::transport::hybrid::{
    HybridAdvertisement, HybridOperationHint, HybridPlaintextMessage,
};
pub use system_hybrid_channel_connector::SystemHybridChannelConnector;
#[cfg(feature = "test-support")]
pub use tunnel::HybridTunnelTestRoute;
pub use tunnel::{HybridTunnelEndpoint, HybridWebSocketError, WebSocketHybridDataChannel};

#[cfg(test)]
pub(crate) mod test_support;
