use crate::{
    HybridAdvertisementRelay, HybridChannelConnection, HybridConnectionError, HybridCtapTransport,
    HybridQrInitiatedTransaction, HybridTunnelEndpoint, WebSocketHybridDataChannel,
};
use futures_util::future::BoxFuture;

use super::ble::{HybridAdvertisementDiscoveryError, discover_advertisement};
use super::hybrid_channel_connector::HybridChannelConnector;
/// A real-world implementation with tunnel connections and Noise handshaking, followed by exploring Hybrid ads.
/// Once the connection is established, it returns a channel that can communicate with the authenticator of the smartphone.
#[derive(Default)]
pub struct SystemHybridChannelConnector {
    remote_advertisements: Option<HybridAdvertisementRelay>,
    #[cfg(feature = "test-support")]
    test_tunnel_route: Option<crate::HybridTunnelTestRoute>,
}

impl SystemHybridChannelConnector {
    pub fn remote_advertisements(mut self, relay: HybridAdvertisementRelay) -> Self {
        self.remote_advertisements = Some(relay);
        self
    }

    #[cfg(feature = "test-support")]
    pub fn test_tunnel_route(mut self, route: crate::HybridTunnelTestRoute) -> Self {
        self.test_tunnel_route = Some(route);
        self
    }
}

impl HybridChannelConnector for SystemHybridChannelConnector {
    fn connect<'a>(
        &'a self,
        transaction: &'a HybridQrInitiatedTransaction,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<HybridChannelConnection, HybridConnectionError>> {
        Box::pin(async move {
            let eid_key = transaction.eid_key()?;
            let advertisement = match discover_advertisement(
                &eid_key,
                self.remote_advertisements.as_ref(),
                cancellation,
            )
            .await
            {
                Ok(advertisement) => advertisement,
                Err(HybridAdvertisementDiscoveryError::Unavailable(reason)) => {
                    return Ok(HybridChannelConnection::Unavailable(reason));
                }
                Err(HybridAdvertisementDiscoveryError::Cancelled) => {
                    return Err(HybridConnectionError::Cancelled);
                }
                Err(HybridAdvertisementDiscoveryError::Ble(error)) => return Err(error.into()),
            };
            let endpoint =
                HybridTunnelEndpoint::from_advertisement(&transaction.tunnel_id()?, &advertisement)
                    .ok_or(HybridConnectionError::InvalidTunnelDomain)?;
            tracing::debug!(domain = %endpoint.domain(), "connecting to hybrid tunnel");
            let data_channel = WebSocketHybridDataChannel::connect(
                &endpoint,
                #[cfg(feature = "test-support")]
                self.test_tunnel_route.as_ref(),
            )
            .await?;
            tracing::debug!("connected to hybrid tunnel; starting Noise handshake");
            let channel = HybridCtapTransport::connect(
                data_channel,
                &transaction.private_key_bytes(),
                &*transaction.psk(advertisement.plaintext())?,
            )
            .await
            .map_err(HybridConnectionError::channel)?;
            tracing::debug!("hybrid Noise handshake completed");
            Ok(HybridChannelConnection::Connected(Box::new(channel)))
        })
    }
}
