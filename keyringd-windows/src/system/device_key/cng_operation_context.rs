#[derive(Clone, Copy, Debug, Default)]
pub struct CngOperationContext {
    parent_window_handle: isize,
}

impl CngOperationContext {
    pub(crate) fn for_window(parent_window_handle: usize) -> Self {
        Self {
            parent_window_handle: parent_window_handle as isize,
        }
    }

    pub(super) fn parent_window_handle(self) -> isize {
        self.parent_window_handle
    }
}
