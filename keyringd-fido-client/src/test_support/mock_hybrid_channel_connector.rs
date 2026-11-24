use crate::{
    HybridChannelConnection, HybridChannelConnector, HybridConnectionError,
    HybridQrInitiatedTransaction,
};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

mockall::mock! {
    /// A mock that specifies the response/cancellation wait for a Hybrid connection for each test.
    pub(crate) HybridChannelConnector {}
    impl HybridChannelConnector for HybridChannelConnector {
        fn connect<'a>(&'a self, transaction: &'a HybridQrInitiatedTransaction, cancellation: CancellationToken)
            -> BoxFuture<'static, Result<HybridChannelConnection, HybridConnectionError>>;
    }
}

pub(crate) fn pending_hybrid_connector() -> MockHybridChannelConnector {
    let mut connector = MockHybridChannelConnector::new();
    connector.expect_connect().returning(|_, cancellation| {
        Box::pin(async move {
            cancellation.cancelled().await;
            Err(HybridConnectionError::Cancelled)
        })
    });
    connector
}
