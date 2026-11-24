use super::{
    HybridAdvertisementDiscoveryError, HybridAdvertisementRelay, HybridBleScanner,
    HybridBleScannerAvailability,
};

use crate::{HybridAdvertisement, HybridSecret64};
use tokio_util::sync::CancellationToken;

pub(crate) async fn discover_advertisement(
    eid_key: &HybridSecret64,
    relay: Option<&HybridAdvertisementRelay>,
    cancellation: CancellationToken,
) -> Result<HybridAdvertisement, HybridAdvertisementDiscoveryError> {
    // Subscribe before starting BLE so forwarded advertisements are not lost during startup.
    let mut remote = relay.map(HybridAdvertisementRelay::subscribe);
    let started = tokio::select! {
        scanner = HybridBleScanner::start() => scanner,
        _ = cancellation.cancelled() => return Err(HybridAdvertisementDiscoveryError::Cancelled),
    };
    let mut scanner = match started {
        Ok(HybridBleScannerAvailability::Available(scanner)) => Some(scanner),
        Ok(HybridBleScannerAvailability::Unavailable(reason)) => {
            if remote.is_none() {
                return Err(HybridAdvertisementDiscoveryError::Unavailable(reason));
            }
            tracing::debug!(?reason, "waiting for remote Hybrid advertisements");
            None
        }
        Err(error) => {
            if remote.is_none() {
                return Err(error.into());
            }
            tracing::warn!(%error, "local BLE scan failed; waiting for remote Hybrid advertisements");
            None
        }
    };
    let result = loop {
        let bytes = tokio::select! {
            result = async {
                match scanner.as_mut() {
                    Some(scanner) => scanner.receive().await,
                    None => std::future::pending().await,
                }
            } => match result {
                Ok(bytes) => bytes,
                Err(error) => {
                    if remote.is_none() {
                        break Err(error.into());
                    }
                    tracing::warn!(%error, "local BLE scan ended; waiting for remote Hybrid advertisements");
                    stop_scanner(scanner.take()).await;
                    continue;
                }
            },
            bytes = receive_remote(remote.as_mut()) => bytes,
            _ = cancellation.cancelled() => break Err(HybridAdvertisementDiscoveryError::Cancelled),
        };
        if let Some(advertisement) = match_advertisement(eid_key, &bytes) {
            break Ok(advertisement);
        }
    };
    stop_scanner(scanner).await;
    result
}

async fn stop_scanner(scanner: Option<HybridBleScanner>) {
    if let Some(scanner) = scanner
        && let Err(error) = scanner.stop().await
    {
        tracing::warn!(%error, "failed to stop Hybrid BLE scan");
    }
}

async fn receive_remote(
    receiver: Option<&mut tokio::sync::broadcast::Receiver<[u8; 20]>>,
) -> Vec<u8> {
    let Some(receiver) = receiver else {
        return std::future::pending().await;
    };
    loop {
        match receiver.recv().await {
            Ok(bytes) => return bytes.to_vec(),
            Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                tracing::trace!(count, "dropped stale remote Hybrid advertisements");
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                return std::future::pending().await;
            }
        }
    }
}

fn match_advertisement(eid_key: &HybridSecret64, bytes: &[u8]) -> Option<HybridAdvertisement> {
    crate::util::wire_trace::trace_bytes("hybrid-advertisement", "in", bytes, None);
    match crate::transport::hybrid::crypto::decrypt_advertisement(eid_key, bytes) {
        Ok(plaintext) => Some(HybridAdvertisement::new(plaintext)),
        Err(error) => {
            tracing::trace!(%error, "ignored non-matching Hybrid advertisement");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "match_advertisement" {
            case "Advertisements are authenticated against the current qr secret" {
                verify "accept the advertisement with the current QR secret and reject it with another secret" {
                    let qr_secret = hex_literal::hex!("000102030405060708090a0b0c0d0e0f");
                    let eid_key = crate::transport::hybrid::crypto::derive(
                        &qr_secret,
                        None,
                        crate::transport::hybrid::crypto::HybridKeyPurpose::Eid,
                    )
                    .unwrap();
                    assert!(match_advertisement(&eid_key, &[0; 20]).is_none());
                    let advertisement = match_advertisement(
                        &eid_key,
                        &hex_literal::hex!("35d007a5bae78986c164f4666e8421909bd69006"),
                    )
                    .unwrap();
                    assert_eq!(
                        advertisement.plaintext(),
                        &hex_literal::hex!("000102030405060708090aaabbcc0000")
                    );
                }
            }
        }
    }
}
