use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::Instrument;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

tokio::task_local! {
    static CURRENT_ACTION: ActionId;
}

/// Identifies the chain of work initiated by one UI action or SSH signing request.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ActionId(u64);

impl ActionId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn generate() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn current() -> Option<Self> {
        CURRENT_ACTION.try_with(|id| *id).ok()
    }

    pub fn span(self) -> tracing::Span {
        tracing::info_span!(target: "keyringd_context", parent: None,
            "action", act_id = self.0, kind = tracing::field::Empty)
    }

    pub fn scope<F: Future>(self, future: F) -> impl Future<Output = F::Output> {
        let span = if Self::current() == Some(self) {
            tracing::Span::none()
        } else {
            self.span()
        };
        CURRENT_ACTION.scope(self, future.instrument(span))
    }

    pub fn sync_scope<T>(self, operation: impl FnOnce() -> T) -> T {
        if Self::current() == Some(self) {
            operation()
        } else {
            CURRENT_ACTION.sync_scope(self, || self.span().in_scope(operation))
        }
    }
}
