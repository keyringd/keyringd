use std::sync::{Arc, Mutex};

use super::TestOperation;

#[derive(Clone, Default)]
pub(crate) struct OperationJournal {
    operations: Arc<Mutex<Vec<TestOperation>>>,
}

impl OperationJournal {
    pub(crate) fn record(&self, operation: TestOperation) {
        self.operations.lock().unwrap().push(operation);
    }

    #[cfg(test)]
    pub(crate) fn clear(&self) {
        self.operations.lock().unwrap().clear();
    }

    #[cfg(test)]
    pub(crate) fn operations(&self) -> Vec<TestOperation> {
        self.operations.lock().unwrap().clone()
    }
}
