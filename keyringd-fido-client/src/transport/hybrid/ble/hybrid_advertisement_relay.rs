use tokio::sync::broadcast;

/// Deliver BLE ads from explicitly enabled external scanners to a connection wait process within the same process.
/// Advertisement/decryption is done by the search side, and Relay is in charge of only delivering data.
/// Previous ads for QR operations will not be redelivered.
#[derive(Clone)]
pub struct HybridAdvertisementRelay {
    sender: broadcast::Sender<[u8; 20]>,
}

impl Default for HybridAdvertisementRelay {
    fn default() -> Self {
        Self::new()
    }
}

impl HybridAdvertisementRelay {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(32);
        Self { sender }
    }

    pub fn publish(&self, service_data: [u8; 20]) {
        // No pending Hybrid operation: discard the advertisement rather than replaying it later.
        let _ = self.sender.send(service_data);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<[u8; 20]> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "HybridAdvertisementRelay::subscribe" {
            case "Only current subscribers receive advertisements" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "deliver advertisements only to receivers subscribed during publication" {
                    let relay = HybridAdvertisementRelay::new();
                    relay.publish([1; 20]);
                    let mut current = relay.subscribe();
                    relay.publish([2; 20]);
                    assert_eq!(current.recv().await.unwrap(), [2; 20]);
                }
            }
        }
    }
}
