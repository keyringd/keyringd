use crate::execution::ExecutionId;
use std::future::Future;

/// Supplies an execution scope when a test calls an interactive service directly.
pub async fn run_test_execution<F: Future>(future: F) -> F::Output {
    ExecutionId::generate().scope(future).await
}
