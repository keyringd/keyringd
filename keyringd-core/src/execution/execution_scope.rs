use crate::execution::ExecutionId;
use std::future::Future;
use std::pin::Pin;

/// Execution-owned identity and logging lifecycle; construction captures the current action.
#[derive(Clone, Copy)]
pub(crate) struct ExecutionScope {
    id: ExecutionId,
    kind: &'static str,
}

impl ExecutionScope {
    pub(crate) fn new(kind: &'static str) -> Self {
        Self {
            id: ExecutionId::generate(),
            kind,
        }
    }

    // Erase the procedure future at the execution boundary so UI futures do not
    // inherit the full FIDO future type and its stack/trait-evaluation depth.
    pub(crate) fn run<'a, T, E>(
        self,
        future: impl Future<Output = Result<T, E>> + Send + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>> {
        Box::pin(self.id.scope(async move {
            tracing::Span::current().record("kind", self.kind);
            let started = std::time::Instant::now();
            tracing::info!("execution started");
            let result = future.await;
            tracing::info!(
                outcome = if result.is_ok() { "ok" } else { "error" },
                elapsed_ms = started.elapsed().as_millis() as u64,
                "execution finished"
            );
            result
        }))
    }

    pub(crate) fn run_sync<T, E>(self, operation: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
        self.id.sync_scope(|| {
            tracing::Span::current().record("kind", self.kind);
            let started = std::time::Instant::now();
            tracing::info!("execution started");
            let result = operation();
            tracing::info!(
                outcome = if result.is_ok() { "ok" } else { "error" },
                elapsed_ms = started.elapsed().as_millis() as u64,
                "execution finished"
            );
            result
        })
    }
}
