use super::TestFidoInteractionHandler;
use keyringd_fido_client::{OperationId, OperationStatus};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) struct TestFidoOperation {
    id: OperationId,
    handler: Option<Arc<dyn TestFidoInteractionHandler>>,
    pending: Arc<AtomicUsize>,
    cancellations: Arc<
        std::sync::Mutex<std::collections::HashMap<OperationId, tokio::sync::watch::Sender<bool>>>,
    >,
}

impl TestFidoOperation {
    pub(super) fn new(
        id: OperationId,
        handler: Option<Arc<dyn TestFidoInteractionHandler>>,
        pending: Arc<AtomicUsize>,
        cancellations: Arc<
            std::sync::Mutex<
                std::collections::HashMap<OperationId, tokio::sync::watch::Sender<bool>>,
            >,
        >,
    ) -> Self {
        Self {
            id,
            handler,
            pending,
            cancellations,
        }
    }
}

impl Drop for TestFidoOperation {
    fn drop(&mut self) {
        self.cancellations.lock().unwrap().remove(&self.id);
        self.pending.fetch_sub(1, Ordering::SeqCst);
        let Some(handler) = self.handler.clone() else {
            return;
        };
        let id = self.id;
        tokio::spawn(async move {
            let _ = handler
                .on_operation_status_changed(OperationStatus::BackendFinished { operation_id: id })
                .await;
        });
    }
}
