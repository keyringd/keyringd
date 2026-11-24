use crate::*;
use crate::{HybridAdvertisement, HybridCtapTransport};

use futures_util::future::BoxFuture;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize};

use super::*;

/// Test connection processing to build a Hybrid connection with the authenticator model.
/// Observe and control the progress and end of communication, and verify the selection of candidates and the aftermath.
pub(crate) struct ModelHybridConnector {
    pub(crate) authenticator: Arc<keyringd_test_authenticator::ModelCtapAuthenticator>,
    pub(crate) closed_channels: Arc<AtomicUsize>,
    pub(crate) shutdown_frames: Arc<AtomicUsize>,
    pub(crate) ctap_entered: Option<Arc<AtomicBool>>,
    pub(crate) resume_ctap: Option<Arc<tokio::sync::Notify>>,
}

impl HybridChannelConnector for ModelHybridConnector {
    fn connect<'a>(
        &'a self,
        transaction: &'a HybridQrInitiatedTransaction,
        _cancellation: tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<HybridChannelConnection, crate::HybridConnectionError>> {
        Box::pin(async move {
            let advertisement = HybridAdvertisement::new([0; 16]);
            let channel = ModelHybridDataChannel::new(
                transaction,
                &advertisement,
                Arc::clone(&self.authenticator),
                Arc::clone(&self.closed_channels),
                Arc::clone(&self.shutdown_frames),
                self.ctap_entered.clone(),
                self.resume_ctap.clone(),
            );
            HybridCtapTransport::connect(
                channel,
                &transaction.private_key_bytes(),
                &*transaction.psk(advertisement.plaintext())?,
            )
            .await
            .map(|channel| Box::new(channel) as Box<dyn crate::CtapTransport>)
            .map(HybridChannelConnection::Connected)
            .map_err(crate::HybridConnectionError::channel)
        })
    }
}
