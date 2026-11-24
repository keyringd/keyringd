use super::PasskeyService;
use keyringd_fido_client::FidoClientError;
use std::future::Future;
use tokio_util::sync::CancellationToken;

/// Serializes complete FIDO interactions, including transport retries and UI cleanup.
pub(crate) struct SerializedPasskeyService<F> {
    service: F,
    queue: tokio::sync::Mutex<()>,
}

impl<F: PasskeyService> SerializedPasskeyService<F> {
    pub(crate) fn new(service: F) -> Self {
        Self {
            service,
            queue: tokio::sync::Mutex::new(()),
        }
    }

    pub(crate) async fn run<'a, T, E, Fut>(
        &'a self,
        operation: impl FnOnce(&'a F) -> Fut + Send,
    ) -> Result<T, E>
    where
        Fut: Future<Output = Result<T, E>> + Send,
    {
        let _turn = self.queue.lock().await;
        operation(&self.service).await
    }

    /// Cancels while queued. Once started, the backend owns cancellation and cleanup.
    pub(crate) async fn run_with_cancellable_wait<'a, T, E, Fut>(
        &'a self,
        cancellation: &CancellationToken,
        operation: impl FnOnce(&'a F) -> Fut + Send,
    ) -> Result<T, E>
    where
        E: From<FidoClientError>,
        Fut: Future<Output = Result<T, E>> + Send,
    {
        let _turn = tokio::select! {
            biased;
            _ = cancellation.cancelled() => return Err(FidoClientError::Cancelled.into()),
            turn = self.queue.lock() => turn,
        };
        if cancellation.is_cancelled() {
            return Err(FidoClientError::Cancelled.into());
        }
        operation(&self.service).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestPasskeyService;
    use futures_util::poll;
    use std::sync::atomic::{AtomicBool, Ordering};

    keyringd_testkit::scenario! {
        group "SerializedPasskeyService" {
            case "Cancels queued work without interrupting the running interaction or its cleanup" {
                why "Dropping an in-flight interaction on cancellation can skip native UI cleanup and let the next request overlap it."
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                verify "the next interaction starts only after the running interaction settles" {
                    let service = SerializedPasskeyService::new(TestPasskeyService::new());
                    let active_cancellation = CancellationToken::new();
                    let (finish, finished) = tokio::sync::oneshot::channel();
                    let first = service.run_with_cancellable_wait(&active_cancellation, |_| async {
                        finished.await.unwrap();
                        Ok::<_, FidoClientError>(())
                    });
                    tokio::pin!(first);
                    assert!(poll!(first.as_mut()).is_pending());

                    let queued_cancellation = CancellationToken::new();
                    let entered = AtomicBool::new(false);
                    let queued = service.run_with_cancellable_wait(&queued_cancellation, |_| async {
                        entered.store(true, Ordering::SeqCst);
                        Ok::<_, FidoClientError>(())
                    });
                    tokio::pin!(queued);
                    assert!(poll!(queued.as_mut()).is_pending());
                    queued_cancellation.cancel();
                    assert!(matches!(queued.await, Err(FidoClientError::Cancelled)));
                    assert!(!entered.load(Ordering::SeqCst));

                    active_cancellation.cancel();
                    assert!(poll!(first.as_mut()).is_pending());
                    let next = service.run(|_| async { Ok::<_, FidoClientError>(()) });
                    tokio::pin!(next);
                    assert!(poll!(next.as_mut()).is_pending());
                    finish.send(()).unwrap();
                    first.await.unwrap();
                    next.await.unwrap();
                }
            }
        }
    }
}
