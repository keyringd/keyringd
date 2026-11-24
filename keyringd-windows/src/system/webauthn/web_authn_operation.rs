use keyringd_fido_client::{FidoClientError, OperationId};
use tokio_util::sync::CancellationToken;
use windows::Win32::Networking::WindowsWebServices::{
    WebAuthNCancelCurrentOperation, WebAuthNGetCancellationId,
};
use windows::core::GUID;

/// Windows WebAuthn one call and manage the lifespan of its canceled ID.
/// It tells the OS that the token has been canceled and waits for the end, and requests the cancellation of the OS even when the Future is discarded.
pub(super) struct WebAuthnOperation {
    operation_id: OperationId,
    cancellation_id: Option<GUID>,
}

impl WebAuthnOperation {
    pub(super) fn new(operation_id: OperationId) -> Result<Self, FidoClientError> {
        let cancellation_id =
            unsafe { WebAuthNGetCancellationId() }.map_err(FidoClientError::backend)?;
        Ok(Self {
            operation_id,
            cancellation_id: Some(cancellation_id),
        })
    }

    pub(super) async fn run<T, F>(
        mut self,
        cancellation: CancellationToken,
        execute: F,
    ) -> Result<T, FidoClientError>
    where
        T: Send + 'static,
        F: FnOnce(GUID, CancellationToken) -> Result<T, FidoClientError> + Send + 'static,
    {
        let cancellation = cancellation.child_token();
        let _cancel_on_drop = cancellation.clone().drop_guard();
        if cancellation.is_cancelled() {
            self.cancellation_id = None;
            return Err(FidoClientError::Cancelled);
        }
        let id = self.cancellation_id.unwrap_or_else(|| GUID::from_u128(0));
        let worker_cancellation = cancellation.clone();
        let span = tracing::Span::current();
        let mut worker =
            tokio::task::spawn_blocking(move || span.in_scope(|| execute(id, worker_cancellation)));
        let result = tokio::select! {
            biased;
            _ = cancellation.cancelled() => {
                self.cancel();
                // Wait until the OS dialog closes and do not discard the calling screen first.
                if tokio::time::timeout(std::time::Duration::from_secs(5), &mut worker).await.is_err() {
                    tracing::warn!(operation_id = ?self.operation_id, "Windows WebAuthn did not finish within 5 seconds after cancellation");
                }
                Err(FidoClientError::Cancelled)
            }
            result = &mut worker => result.map_err(FidoClientError::backend)?,
        };
        self.cancellation_id = None;
        result
    }

    fn cancel(&mut self) {
        if let Some(id) = self.cancellation_id.take()
            && let Err(error) = unsafe { WebAuthNCancelCurrentOperation(&id) }
        {
            tracing::warn!(operation_id = ?self.operation_id, ?id, %error, "Windows WebAuthn cancellation failed");
        }
    }
}

impl Drop for WebAuthnOperation {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn test_operation() -> WebAuthnOperation {
        WebAuthnOperation {
            operation_id: OperationId::new(1),
            cancellation_id: None,
        }
    }

    keyringd_testkit::scenario! {
        group "WebAuthnOperation::cancel" {
            case "Cancelled before start does not launch worker" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "leave the native worker unstarted when cancellation is already set" {
                    let cancellation = CancellationToken::new();
                    cancellation.cancel();
                    let result = test_operation()
                        .run::<(), _>(cancellation, |_, _| {
                            panic!("cancelled operation must not start");
                        })
                        .await;
                    assert!(matches!(result, Err(FidoClientError::Cancelled)));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "WebAuthnOperation::cancel" {
            case "Cancellation waits for worker cleanup" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "wait for the native worker to finish cleaning up after cancellation" {
                    let cancellation = CancellationToken::new();
                    let worker_cancellation = cancellation.clone();
                    let (started_sender, started) = tokio::sync::oneshot::channel();
                    let (cancelled_sender, cancelled) = tokio::sync::oneshot::channel();
                    let (cleanup_sender, cleanup) = std::sync::mpsc::channel();
                    let settled = Arc::new(AtomicBool::new(false));
                    let worker_settled = Arc::clone(&settled);
                    let task = tokio::spawn(async move {
                        test_operation()
                            .run(worker_cancellation, move |_, cancellation| {
                                started_sender.send(()).unwrap();
                                tokio::runtime::Handle::current().block_on(cancellation.cancelled());
                                cancelled_sender.send(()).unwrap();
                                cleanup.recv().unwrap();
                                worker_settled.store(true, Ordering::SeqCst);
                                Err::<(), _>(FidoClientError::Cancelled)
                            })
                            .await
                    });
                    started.await.unwrap();
                    cancellation.cancel();
                    cancelled.await.unwrap();
                    assert!(!task.is_finished());
                    cleanup_sender.send(()).unwrap();
                    let result = task.await.unwrap();
                    assert!(matches!(result, Err(FidoClientError::Cancelled)));
                    assert!(settled.load(Ordering::SeqCst));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "WebAuthnOperation::run" {
            case "Dropping future cancels worker without cancelling parent" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "signal the worker when its future is dropped and keep the parent token active" {
                    let cancellation = CancellationToken::new();
                    let worker_cancellation = cancellation.clone();
                    let (started_sender, started) = tokio::sync::oneshot::channel();
                    let (settled_sender, settled) = tokio::sync::oneshot::channel();
                    let task = tokio::spawn(async move {
                        test_operation()
                            .run(worker_cancellation, move |_, cancellation| {
                                started_sender.send(()).unwrap();
                                tokio::runtime::Handle::current().block_on(cancellation.cancelled());
                                settled_sender.send(()).unwrap();
                                Err::<(), _>(FidoClientError::Cancelled)
                            })
                            .await
                    });
                    started.await.unwrap();
                    task.abort();
                    assert!(task.await.unwrap_err().is_cancelled());
                    tokio::time::timeout(std::time::Duration::from_secs(1), settled)
                        .await
                        .unwrap()
                        .unwrap();
                    assert!(!cancellation.is_cancelled());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "WebAuthnOperation::run" {
            case "Completion returns worker result without cancelling parent" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "return the worker's output and leave the parent token active" {
                    let cancellation = CancellationToken::new();
                    let result = test_operation()
                        .run(cancellation.clone(), |_, _| Ok(7))
                        .await;
                    assert_eq!(result.unwrap(), 7);
                    assert!(!cancellation.is_cancelled());
                }
            }
        }
    }
}
