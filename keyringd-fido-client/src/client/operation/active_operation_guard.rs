use std::sync::Mutex;

use crate::OperationId;

use tokio_util::sync::CancellationToken;

/// A guard that manages how long OperationManager keeps the operation running.
/// End of operation or run When the Future is discarded, the hold state is canceled so that the next operation can be accepted.
pub(in crate::client::operation) struct ActiveOperationGuard<'a> {
    pub(in crate::client::operation) operation_id: OperationId,
    pub(in crate::client::operation) active_operation:
        &'a Mutex<Option<(OperationId, CancellationToken)>>,
}

impl Drop for ActiveOperationGuard<'_> {
    fn drop(&mut self) {
        let mut active_operation = self
            .active_operation
            .lock()
            .expect("FIDO active operation lock poisoned");
        if active_operation
            .as_ref()
            .is_some_and(|(id, _)| *id == self.operation_id)
        {
            *active_operation = None;
        }
    }
}
