use crate::application::ActionId;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::Instrument;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

tokio::task_local! {
    static CURRENT_EXECUTION: ExecutionId;
}

/// Identifies one execution and the action that initiated it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ExecutionId {
    value: u64,
    action_id: ActionId,
}

impl ExecutionId {
    pub fn generate() -> Self {
        Self {
            value: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            action_id: ActionId::current().unwrap_or_else(ActionId::generate),
        }
    }

    pub const fn get(self) -> u64 {
        self.value
    }

    pub const fn action_id(self) -> ActionId {
        self.action_id
    }

    pub fn current() -> Option<Self> {
        CURRENT_EXECUTION.try_with(|id| *id).ok()
    }

    /// Reconstructs the log context when an event crosses a task or UI boundary.
    pub fn span(self) -> tracing::Span {
        tracing::info_span!(target: "keyringd_context", parent: None,
            "exec", exec_id = self.value, act_id = self.action_id.get(), kind = tracing::field::Empty)
    }

    /// Carries a caller's operation identity through the polled future.
    pub fn scope<F: Future>(self, future: F) -> impl Future<Output = F::Output> {
        let span = if Self::current() == Some(self) {
            tracing::Span::none()
        } else {
            self.span()
        };
        self.action_id
            .scope(CURRENT_EXECUTION.scope(self, future.instrument(span)))
    }

    /// Carries the same identity into synchronous callbacks or blocking workers.
    pub fn sync_scope<T>(self, operation: impl FnOnce() -> T) -> T {
        let execute = || {
            if Self::current() == Some(self) {
                operation()
            } else {
                CURRENT_EXECUTION.sync_scope(self, || self.span().in_scope(operation))
            }
        };
        self.action_id.sync_scope(execute)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "ExecutionId::generate" {
            case "Assigns distinct IDs to executions of the current action" {
                verify "generated executions have different IDs and retain the current action" {
                    let action = ActionId::generate();
                    action.sync_scope(|| {
                        let first = ExecutionId::generate();
                        let second = ExecutionId::generate();
                        assert_ne!(first.get(), second.get());
                        assert_eq!(first.action_id(), action);
                        assert_eq!(second.action_id(), action);
                    });
                }
            }
        }
        group "ExecutionId::sync_scope" {
            case "Makes the execution current only for the duration of its scope" {
                verify "the scoped execution is available inside the callback and cleared after it returns" {
                    let execution = ExecutionId::generate();
                    execution.sync_scope(|| assert_eq!(ExecutionId::current(), Some(execution)));
                    assert!(ExecutionId::current().is_none());
                }
            }
        }
    }
}
