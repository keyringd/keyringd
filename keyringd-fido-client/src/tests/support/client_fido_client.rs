use crate::OperationId;
use crate::backend::OsUsbBackend;
use crate::client::AuthenticatorBroker;
use crate::client::FidoClient;
use crate::client::FidoInteractionHandler;
use crate::client::OperationManager;
use crate::client::backend::UsbBackend;
use crate::client::test_support::OsBackendCalls;
use crate::test_support::pending_hybrid_connector;
use std::sync::Arc;

pub(crate) fn os_test_client<H>(handler: H, backend: Arc<dyn OsUsbBackend>) -> FidoClient<H>
where
    H: FidoInteractionHandler,
{
    FidoClient {
        operation_manager: OperationManager::new(handler),
        broker: AuthenticatorBroker::new(
            Arc::new(pending_hybrid_connector()),
            UsbBackend::Os(backend),
        ),
        internal_authenticator: (),
    }
}

pub(crate) async fn wait_for_active_backend(backend: &OsBackendCalls) -> OperationId {
    loop {
        if let Some(operation_id) = *backend.active.lock().unwrap() {
            return operation_id;
        }
        tokio::task::yield_now().await;
    }
}
