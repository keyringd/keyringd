/// Reasons to stop exploration of Hybrid ads. Distinguish between unavailable, canceled and BLE receive errors.
#[derive(Debug, thiserror::Error)]
pub(crate) enum HybridAdvertisementDiscoveryError {
    #[error("hybrid advertisement discovery was cancelled")]
    Cancelled,
    #[error("hybrid advertisement discovery is unavailable: {0:?}")]
    Unavailable(crate::HybridTransportUnavailableReason),
    #[error(transparent)]
    Ble(#[from] crate::HybridBleError),
}
