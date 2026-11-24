use keyringd_core::execution::ExecutionId;

use super::KeyOperationCoordinator;
use keyringd_fido_client::OperationId;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

/// Keeps transport retries bound to the caller's execution and waits for each
/// attempt's UI dismissal. The caller owns the action that routes those interactions.
pub struct FidoOperationContext {
    pub usb: watch::Sender<bool>,
    pub active: Mutex<Option<OperationId>>,
    pub execution_id: ExecutionId,
    pub cancellation: CancellationToken,
    handler: Arc<KeyOperationCoordinator>,
}

tokio::task_local! {
    pub(super) static CURRENT_FIDO_OPERATION: Arc<FidoOperationContext>;
}

impl FidoOperationContext {
    pub fn new(handler: &Arc<KeyOperationCoordinator>) -> Arc<Self> {
        let execution_id =
            ExecutionId::current().expect("FIDO interaction requires an application execution");
        let action_id = execution_id.action_id();
        let cancellation = handler
            .actions
            .cancellation(action_id)
            .expect("the action interaction route must remain active");
        Arc::new(Self {
            usb: watch::channel(false).0,
            active: Mutex::new(None),
            execution_id,
            cancellation,
            handler: Arc::clone(handler),
        })
    }

    pub async fn scope<F: std::future::Future>(self: &Arc<Self>, future: F) -> F::Output {
        ExecutionId::scope(
            self.execution_id,
            CURRENT_FIDO_OPERATION.scope(Arc::clone(self), future),
        )
        .await
    }

    /// Dismisses this attempt's UI after its backend future has settled.
    pub(super) async fn dismiss_attempt_ui(&self) {
        let id = self
            .active
            .lock()
            .expect("FIDO operation mutex must be unpoisoned")
            .take();
        if let Some(id) = id {
            self.handler.dismiss_fido_ui(id).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_core::application::PasskeyChoice;

    keyringd_testkit::scenario! {
        group "FidoOperationContext::scope" {
            case "Fido contexts share the execution without ending the action route" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "preserve the execution ID across FIDO contexts and keep the action registered" {
                    let (handle, _events) = KeyOperationCoordinator::channel();
                    let action = handle.begin_profile_setup(PasskeyChoice::Existing);
                    let id = action.id();
                    id.scope(keyringd_core::test_support::run_test_execution(async {
                        let execution_id = ExecutionId::current().unwrap();
                        for _ in 0..2 {
                            let context = FidoOperationContext::new(&handle);
                            assert_eq!(context.execution_id, execution_id);
                            context
                                .scope(async {
                                    CURRENT_FIDO_OPERATION.with(|current| {
                                        assert!(Arc::ptr_eq(current, &context));
                                    });
                                    assert_eq!(ExecutionId::current(), Some(execution_id));
                                })
                                .await;
                            drop(context);
                            assert!(handle.actions.contains(id));
                        }
                    }))
                    .await;
                    assert!(ExecutionId::current().is_none());
                    drop(action);
                    assert!(!handle.actions.contains(id));
                }
            }
        }
    }
}
