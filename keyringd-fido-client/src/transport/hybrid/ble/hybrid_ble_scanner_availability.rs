use crate::{HybridBleScanner, HybridTransportUnavailableReason};

/// Result of starting a BLE scan. Distinguish the available scanners from unavailable reasons such as the absence of an adapter.
pub(crate) enum HybridBleScannerAvailability {
    Available(HybridBleScanner),
    Unavailable(HybridTransportUnavailableReason),
}
