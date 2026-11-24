use super::advertisement_events::AdvertisementEvents;
use tracing::Instrument;

use btleplug::api::{Central, CentralState, Manager as _, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use uuid::Uuid;

use crate::{HybridBleError, HybridBleScannerAvailability, HybridTransportUnavailableReason};

pub(crate) const FIDO_HYBRID_SERVICE: Uuid =
    Uuid::from_u128(0x0000fff9_0000_1000_8000_00805f9b34fb);
// Deprecated caBLE UUID still used by some Google Play services versions.
pub(crate) const GOOGLE_CABLE_SERVICE: Uuid =
    Uuid::from_u128(0x0000fde2_0000_1000_8000_00805f9b34fb);

/// Start a BLE scan and receive the encrypted ad data for the Hybrid service.
/// Preserve the rights of adapters, ad reception, scans, and stop scanning and receiving processing at the end.
/// Exclusive rights are retained until the stop processing is finished and will not be carried over to tunnel connections or CTAP operations.
pub struct HybridBleScanner {
    scan_permit: Option<tokio::sync::MutexGuard<'static, ()>>,
    adapter: Adapter,
    advertisements: AdvertisementEvents,
    scanning: bool,
    advertisements_seen: usize,
}

impl HybridBleScanner {
    pub async fn start() -> Result<HybridBleScannerAvailability, HybridBleError> {
        static BLE_SCAN: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
        let scan_permit = BLE_SCAN
            .get_or_init(|| tokio::sync::Mutex::new(()))
            .lock()
            .await;

        #[cfg(target_os = "linux")]
        if !linux_hci_controller_present().map_err(HybridBleError::platform)? {
            return Ok(HybridBleScannerAvailability::Unavailable(
                HybridTransportUnavailableReason::NoBluetoothAdapter,
            ));
        }
        let manager = Manager::new().await.map_err(HybridBleError::platform)?;
        let adapters = manager.adapters().await.map_err(HybridBleError::platform)?;
        let mut states = Vec::with_capacity(adapters.len());
        for (adapter_index, adapter) in adapters.iter().enumerate() {
            let adapter_info = adapter
                .adapter_info()
                .await
                .unwrap_or_else(|error| format!("unavailable: {error}"));
            tracing::debug!(adapter_index, %adapter_info, "reading Bluetooth adapter state");
            states.push(
                adapter
                    .adapter_state()
                    .await
                    .map_err(HybridBleError::platform)?,
            );
        }
        if let Some(reason) = unavailable_reason(&states) {
            return Ok(HybridBleScannerAvailability::Unavailable(reason));
        }

        let Some(adapter) = adapters
            .into_iter()
            .zip(states)
            .find_map(|(adapter, state)| (state != CentralState::PoweredOff).then_some(adapter))
        else {
            return Ok(HybridBleScannerAvailability::Unavailable(
                HybridTransportUnavailableReason::BluetoothDisabled,
            ));
        };
        let adapter_info = adapter
            .adapter_info()
            .await
            .unwrap_or_else(|error| format!("unavailable: {error}"));
        tracing::debug!(%adapter_info, "selected Bluetooth adapter for hybrid discovery");

        // Subscribe before starting discovery so no short-lived advertisement can fall into a
        // setup gap. btleplug's Linux backend and libwebauthn use one event stream for one
        // adapter; sharing a BlueZ session between concurrent adapter streams is not reliable.
        let advertisements = AdvertisementEvents::subscribe(&adapter).await?;

        tracing::debug!("starting Bluetooth discovery");
        // BlueZ does not consistently match UUID discovery filters against service-data-only
        // caBLE advertisements. Match the UUIDs in-process, as libwebauthn does.
        if let Err(error) = adapter.start_scan(ScanFilter::default()).await {
            return Err(HybridBleError::platform(error));
        }
        tracing::debug!("Bluetooth discovery started");

        Ok(HybridBleScannerAvailability::Available(Self {
            scan_permit: Some(scan_permit),
            adapter,
            advertisements,
            scanning: true,
            advertisements_seen: 0,
        }))
    }

    pub(super) async fn receive(&mut self) -> Result<Vec<u8>, HybridBleError> {
        while let Some(mut service_data) = self.advertisements.receive().await {
            if let Some(bytes) = service_data
                .remove(&FIDO_HYBRID_SERVICE)
                .or_else(|| service_data.remove(&GOOGLE_CABLE_SERVICE))
            {
                self.advertisements_seen += 1;
                return Ok(bytes);
            }
        }
        Err(HybridBleError::platform(std::io::Error::other(
            "Bluetooth advertisement subscriptions ended while scanning",
        )))
    }

    pub async fn stop(mut self) -> Result<(), HybridBleError> {
        let result = self.adapter.stop_scan().await;
        self.scanning = false;
        self.advertisements.shutdown().await;
        result.map_err(HybridBleError::platform)?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn linux_hci_controller_present() -> std::io::Result<bool> {
    match std::fs::read_dir("/sys/class/bluetooth") {
        Ok(entries) => {
            for entry in entries {
                if entry?.file_name().to_string_lossy().starts_with("hci") {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn unavailable_reason(states: &[CentralState]) -> Option<HybridTransportUnavailableReason> {
    if states.is_empty() {
        Some(HybridTransportUnavailableReason::NoBluetoothAdapter)
    } else if states
        .iter()
        .all(|state| *state == CentralState::PoweredOff)
    {
        Some(HybridTransportUnavailableReason::BluetoothDisabled)
    } else {
        None
    }
}

impl Drop for HybridBleScanner {
    fn drop(&mut self) {
        if !self.scanning {
            return;
        }
        tracing::debug!(
            advertisements_seen = self.advertisements_seen,
            "hybrid BLE scan dropped before being stopped"
        );
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let adapter = self.adapter.clone();
        let scan_permit = self.scan_permit.take();
        runtime.spawn(
            async move {
                let _scan_permit = scan_permit;
                if let Err(error) = adapter.stop_scan().await {
                    tracing::warn!(%error, "failed to stop abandoned hybrid BLE scan");
                }
            }
            .in_current_span(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "unavailable_reason" {
            case "Distinguishes a missing Bluetooth adapter from a disabled adapter" {
                why "The scanner exposes distinct recovery paths for missing hardware and a disabled adapter; this local diagnostic contract is outside CTAP Hybrid framing."
                given "Prepare a Central State with no Adapter, Power Off, Power On, or Unknown Status" {
                    let states = [
                        Vec::new(),
                        vec![CentralState::PoweredOff],
                        vec![CentralState::PoweredOn],
                        vec![CentralState::Unknown],
                    ];
                }
                when "Determine the reasons why Hybrid Transport is not available from each State" {
                    let reasons = states.map(|state| unavailable_reason(&state));
                }
                then "Report apart from the Adapter absent from the bluetooth invalid" {
                    assert_eq!(
                        reasons[0],
                        Some(HybridTransportUnavailableReason::NoBluetoothAdapter)
                    );
                    assert_eq!(
                        reasons[1],
                        Some(HybridTransportUnavailableReason::BluetoothDisabled)
                    );
                    assert_eq!(reasons[2], None);
                    assert_eq!(reasons[3], None);
                }
            }
        }
    }
}
