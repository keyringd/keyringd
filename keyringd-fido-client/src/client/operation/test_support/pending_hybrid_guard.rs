use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A test guard to record that the Hybrid connection wait has been discarded.
pub(crate) struct PendingHybridGuard(pub(crate) Arc<AtomicBool>);

impl Drop for PendingHybridGuard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
