use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::Instrument;

use crate::{FidoClientError, FidoInteractionHandler, OperationId, OperationStatus};

use super::ActiveOperationGuard;
use super::operation_context::OperationContext;

/// Manage the issuance of operation ID, registration of cancellation, refusal of concurrent execution, start/end notifications collectively.
/// Receives the execution of the authenticator and handles the life of the operation from start to finish.
pub(crate) struct OperationManager<H> {
    handler: H,
    next_operation_id: AtomicU64,
    active_operation: std::sync::Mutex<Option<(OperationId, tokio_util::sync::CancellationToken)>>,
}

impl<H: FidoInteractionHandler> OperationManager<H> {
    pub(crate) fn new(handler: H) -> Self {
        Self {
            handler,
            next_operation_id: AtomicU64::new(1),
            active_operation: std::sync::Mutex::new(None),
        }
    }
    pub(crate) fn cancel(&self, operation_id: OperationId) -> bool {
        let cancellation = self
            .active_operation
            .lock()
            .expect("FIDO active operation lock poisoned")
            .as_ref()
            .filter(|(id, _)| *id == operation_id)
            .map(|(_, cancellation)| cancellation.clone());
        if let Some(cancellation) = cancellation {
            cancellation.cancel();
            true
        } else {
            false
        }
    }

    pub(crate) async fn run_operation<'a, T, F, Fut>(
        &'a self,
        deadline: std::time::Instant,
        operation: F,
    ) -> Result<T, FidoClientError>
    where
        F: FnOnce(OperationContext<'a>) -> Fut,
        Fut: Future<Output = Result<T, FidoClientError>>,
    {
        let (context, _active_operation) = {
            let mut active_operation = self
                .active_operation
                .lock()
                .expect("FIDO active operation lock poisoned");
            if active_operation.is_some() {
                return Err(FidoClientError::Busy);
            }
            let id = OperationId::new(self.next_operation_id.fetch_add(1, Ordering::Relaxed));
            let cancellation = tokio_util::sync::CancellationToken::new();
            let context = OperationContext {
                id,
                deadline,
                cancellation: cancellation.clone(),
                handler: &self.handler,
            };
            *active_operation = Some((id, cancellation));
            (
                context,
                ActiveOperationGuard {
                    operation_id: id,
                    active_operation: &self.active_operation,
                },
            )
        };
        let operation_id = context.id;
        async {
            tracing::debug!("FIDO operation started");
            self.handler
                .on_operation_status_changed(OperationStatus::OperationStarted { operation_id })
                .await?;
            let result = if context.cancellation.is_cancelled() {
                Err(FidoClientError::Cancelled)
            } else if std::time::Instant::now() >= deadline {
                Err(FidoClientError::TimedOut)
            } else {
                operation(context).await
            };
            let finished = self
                .handler
                .on_operation_status_changed(OperationStatus::BackendFinished { operation_id })
                .await;
            if result.is_err()
                && let Err(error) = &finished
            {
                tracing::warn!(
                    ?error,
                    "FIDO completion notification failed after an operation failure"
                );
            }
            match result {
                Err(error) => Err(error),
                Ok(value) => {
                    finished?;
                    Ok(value)
                }
            }
        }
        .instrument(operation_id.span())
        .await
    }
}
