use super::super::AuthenticatorBroker;
use super::super::operation_context::OperationContext;
use super::super::operation_manager::OperationManager;
use crate::backend::UsbBackend;
use crate::client::PreparedMakeCredential;
use crate::*;

use std::sync::Arc;
use std::time::{Duration, Instant};

/// A test client that combines actual operation management with the adjustment of candidates for authenticating device while replacing the communication processing.
pub(crate) struct BrokerTestClient<H> {
    pub(crate) operation_manager: OperationManager<H>,
    pub(crate) broker: AuthenticatorBroker,
}

impl<H: FidoInteractionHandler> BrokerTestClient<H> {
    pub(crate) fn new(handler: H) -> Self {
        Self::with_hybrid_connector(handler, Arc::new(SystemHybridChannelConnector::default()))
    }
}

impl<H: FidoInteractionHandler> BrokerTestClient<H> {
    pub(crate) fn with_hybrid_connector(
        handler: H,
        connector: Arc<dyn HybridChannelConnector>,
    ) -> Self {
        Self {
            operation_manager: OperationManager::new(handler),
            broker: AuthenticatorBroker::new(
                connector,
                UsbBackend::Os(crate::client::test_support::pending_os_backend(Arc::new(
                    crate::client::test_support::OsBackendCalls::default(),
                ))),
            ),
        }
    }
    pub(crate) async fn make_credential(
        &self,
        input: PreparedMakeCredential,
    ) -> Result<MakeCredentialResult, FidoClientError> {
        self.make_credential_with_deadline(input, Instant::now() + Duration::from_secs(1))
            .await
    }
    pub(crate) async fn make_credential_with_deadline(
        &self,
        input: PreparedMakeCredential,
        deadline: Instant,
    ) -> Result<MakeCredentialResult, FidoClientError> {
        self.operation_manager
            .run_operation(deadline, |context| async move {
                let selection = TransportSelection::from_hints(input.options.hints())?;
                self.broker
                    .make_credential(
                        MakeCredentialOperationContext::new(&input, context, selection, None),
                        &(),
                    )
                    .await
            })
            .await
    }
    pub(crate) fn cancel(&self, id: OperationId) -> bool {
        self.operation_manager.cancel(id)
    }
    pub(crate) async fn run_operation<'a, T, F, Fut>(
        &'a self,
        deadline: Instant,
        execute: F,
    ) -> Result<T, FidoClientError>
    where
        F: FnOnce(OperationContext<'a>) -> Fut,
        Fut: Future<Output = Result<T, FidoClientError>>,
    {
        self.operation_manager
            .run_operation(deadline, execute)
            .await
    }
}
