use super::hybrid_ble_scanner::{FIDO_HYBRID_SERVICE, GOOGLE_CABLE_SERVICE};
use crate::HybridBleError;
use btleplug::api::{Central, CentralEvent};
use btleplug::platform::Adapter;
use futures_util::StreamExt;
use std::collections::HashMap;
use tracing::Instrument;
use uuid::Uuid;
const ADVERTISEMENT_QUEUE_CAPACITY: usize = 32;

/// Task to read one Bluetooth adapter event and hold a queue to receive ads.
/// Extract service data related to HYBRID and stop the read task when finished.
pub(super) struct AdvertisementEvents {
    receiver: tokio::sync::mpsc::Receiver<HashMap<Uuid, Vec<u8>>>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl AdvertisementEvents {
    pub(super) async fn subscribe(adapter: &Adapter) -> Result<Self, HybridBleError> {
        tracing::debug!("subscribing to Bluetooth adapter events");
        let mut events = adapter.events().await.map_err(HybridBleError::platform)?;
        tracing::debug!("subscribed to Bluetooth adapter events");

        let (sender, receiver) = tokio::sync::mpsc::channel(ADVERTISEMENT_QUEUE_CAPACITY);
        let task = tokio::spawn(async move {
            let mut event_count = 0usize;
            tracing::debug!("Bluetooth adapter event reader started");
            while let Some(event) = events.next().await {
                event_count += 1;
                if event_count <= 20 || event_count.is_multiple_of(100) {
                    tracing::trace!(
                        event_count,
                        event_kind = central_event_kind(&event),
                        "received Bluetooth adapter event"
                    );
                }
                let service_data = match event {
                    CentralEvent::ServiceDataAdvertisement {
                        mut service_data, ..
                    } => {
                        tracing::trace!(
                        entries = service_data.len(),
                        service_uuids = ?service_data.keys().collect::<Vec<_>>(),
                        payload_lengths = ?service_data.values().map(Vec::len).collect::<Vec<_>>(),
                        "received Bluetooth service-data advertisement"
                        );
                        service_data.retain(|uuid, _| {
                            *uuid == FIDO_HYBRID_SERVICE || *uuid == GOOGLE_CABLE_SERVICE
                        });
                        (!service_data.is_empty()).then_some(service_data)
                    }
                    _ => None,
                };
                if let Some(service_data) = service_data
                    && sender.send(service_data).await.is_err()
                {
                    return;
                }
            }
            tracing::warn!(event_count, "Bluetooth adapter event subscription ended");
        }.in_current_span());

        Ok(Self {
            receiver,
            task: Some(task),
        })
    }

    pub(super) async fn receive(&mut self) -> Option<HashMap<Uuid, Vec<u8>>> {
        self.receiver.recv().await
    }

    pub(super) async fn shutdown(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
            let _ = task.await;
        }
    }
}

fn central_event_kind(event: &CentralEvent) -> &'static str {
    match event {
        CentralEvent::DeviceDiscovered(_) => "device-discovered",
        CentralEvent::DeviceUpdated(_) => "device-updated",
        CentralEvent::DeviceConnected(_) => "device-connected",
        CentralEvent::DeviceDisconnected(_) => "device-disconnected",
        CentralEvent::ManufacturerDataAdvertisement { .. } => "manufacturer-data",
        CentralEvent::ServiceDataAdvertisement { .. } => "service-data",
        CentralEvent::ServicesAdvertisement { .. } => "services",
        CentralEvent::StateUpdate(_) => "adapter-state",
        CentralEvent::DeviceServicesModified(_) => "device-services-modified",
        CentralEvent::RssiUpdate { .. } => "rssi-update",
    }
}

impl Drop for AdvertisementEvents {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
