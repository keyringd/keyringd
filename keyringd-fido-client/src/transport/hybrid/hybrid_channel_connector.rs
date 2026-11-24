use crate::{HybridChannelConnection, HybridConnectionError, HybridQrInitiatedTransaction};
use futures_util::future::BoxFuture;
/// An interface to establish a Hybrid connection with a smartphone that started with a QR.
/// Returns a connected CTAP channel, or a HYBRID unavailable result.
/// The implementation side is in charge of creating/initializing the connection state. Responsible for registration and authentication execution.
/// Treat production connection processing and test implementations at the same entrance.
pub(crate) trait HybridChannelConnector: Send + Sync {
    fn connect<'a>(
        &'a self,
        transaction: &'a HybridQrInitiatedTransaction,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<HybridChannelConnection, HybridConnectionError>>;
}
