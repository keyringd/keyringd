//! BLE search for ads and relay ads received from outside.
mod advertisement_events;
mod hybrid_advertisement_discovery_error;
mod hybrid_advertisement_relay;
mod hybrid_ble_error;
mod hybrid_ble_scanner;
mod hybrid_ble_scanner_availability;

pub(crate) use hybrid_advertisement_discovery_error::HybridAdvertisementDiscoveryError;
pub use hybrid_advertisement_relay::HybridAdvertisementRelay;
pub use hybrid_ble_error::HybridBleError;
pub use hybrid_ble_scanner::HybridBleScanner;
pub(crate) use hybrid_ble_scanner_availability::HybridBleScannerAvailability;

mod advertisement_discovery;
pub(crate) use advertisement_discovery::discover_advertisement;

#[cfg(test)]
pub(crate) use hybrid_ble_scanner::FIDO_HYBRID_SERVICE;

#[cfg(test)]
pub(crate) use hybrid_ble_scanner::GOOGLE_CABLE_SERVICE;
