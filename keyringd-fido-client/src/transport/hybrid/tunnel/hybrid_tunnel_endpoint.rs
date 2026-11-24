use crate::HybridAdvertisement;
use crate::util::hex;

/// The domain routing ID of the decrypted ad and the destination that holds the tunnel ID derived from the QR.
/// The ID is kept as a byte string and converted to hexadecimal notation when creating a websocket destination URL.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridTunnelEndpoint {
    domain: String,
    routing_id: [u8; 3],
    tunnel_id: [u8; 16],
}

impl HybridTunnelEndpoint {
    pub fn from_advertisement(
        tunnel_id: &[u8; 16],
        advertisement: &HybridAdvertisement,
    ) -> Option<Self> {
        Some(Self {
            domain: advertisement.tunnel_server_domain()?,
            routing_id: *advertisement.routing_id(),
            tunnel_id: *tunnel_id,
        })
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn url(&self) -> String {
        format!(
            "wss://{}/cable/connect/{}/{}",
            self.domain,
            hex::encode(&self.routing_id),
            hex::encode(&self.tunnel_id),
        )
    }
}
