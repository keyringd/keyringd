use std::error::Error;
use std::time::Duration;

use btleplug::api::{Central as _, CentralEvent, Manager as _, ScanFilter};
use btleplug::platform::Manager;
use futures_util::StreamExt;
use uuid::Uuid;

const FIDO_HYBRID_SERVICE: Uuid = Uuid::from_u128(0x0000fff9_0000_1000_8000_00805f9b34fb);

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "direct".to_owned());
    let requested = std::env::args().nth(2).unwrap_or_else(|| "hci1".to_owned());
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let mut selected = None;
    for adapter in adapters {
        let info = adapter.adapter_info().await?;
        println!("FOUND {info}");
        if info.starts_with(&requested) {
            selected = Some((adapter, info));
        }
    }
    let (adapter, info) = selected.ok_or_else(|| format!("adapter {requested} not found"))?;

    // This is deliberately the same ordering and consumption model as libwebauthn:
    // subscribe, start an unfiltered scan, then directly poll the stream in this task and ignore
    // every event except ServiceDataAdvertisement.
    let events = adapter.events().await?;
    let target = match mode.as_str() {
        "direct" => {
            adapter.start_scan(ScanFilter::default()).await?;
            println!("PROBE_READY mode=direct {info}");
            receive_direct(events).await
        }
        "forwarded" => {
            let mut service_data = forward_service_data(events);
            adapter.start_scan(ScanFilter::default()).await?;
            println!("PROBE_READY mode=forwarded {info}");
            receive_forwarded(&mut service_data).await
        }
        _ => return Err(format!("unknown mode {mode}").into()),
    };

    let _ = adapter.stop_scan().await;
    match target {
        Ok(true) => Ok(()),
        Ok(false) => Err("Bluetooth event stream ended before the target advertisement".into()),
        Err(_) => Err("timed out waiting for the target advertisement".into()),
    }
}

async fn receive_direct(
    mut events: std::pin::Pin<Box<dyn futures_util::Stream<Item = CentralEvent> + Send>>,
) -> Result<bool, tokio::time::error::Elapsed> {
    tokio::time::timeout(Duration::from_secs(60), async {
        let mut event_count = 0usize;
        while let Some(event) = events.next().await {
            event_count += 1;
            if let CentralEvent::ServiceDataAdvertisement { service_data, .. } = event {
                let services = service_data
                    .iter()
                    .map(|(uuid, payload)| format!("{uuid}:{}", payload.len()))
                    .collect::<Vec<_>>();
                println!("SERVICE_DATA {}", services.join(","));
                if service_data.contains_key(&FIDO_HYBRID_SERVICE) {
                    println!("TARGET_RECEIVED events={event_count}");
                    return true;
                }
            }
        }
        false
    })
    .await
}

fn forward_service_data(
    mut events: std::pin::Pin<Box<dyn futures_util::Stream<Item = CentralEvent> + Send>>,
) -> tokio::sync::mpsc::UnboundedReceiver<std::collections::HashMap<Uuid, Vec<u8>>> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(event) = events.next().await {
            if let CentralEvent::ServiceDataAdvertisement { service_data, .. } = event
                && sender.send(service_data).is_err()
            {
                return;
            }
        }
    });
    receiver
}

async fn receive_forwarded(
    service_data: &mut tokio::sync::mpsc::UnboundedReceiver<
        std::collections::HashMap<Uuid, Vec<u8>>,
    >,
) -> Result<bool, tokio::time::error::Elapsed> {
    tokio::time::timeout(Duration::from_secs(60), async {
        let mut event_count = 0usize;
        while let Some(service_data) = service_data.recv().await {
            event_count += 1;
            let services = service_data
                .iter()
                .map(|(uuid, payload)| format!("{uuid}:{}", payload.len()))
                .collect::<Vec<_>>();
            println!("SERVICE_DATA {}", services.join(","));
            if service_data.contains_key(&FIDO_HYBRID_SERVICE) {
                println!("TARGET_RECEIVED events={event_count}");
                return true;
            }
        }
        false
    })
    .await
}
