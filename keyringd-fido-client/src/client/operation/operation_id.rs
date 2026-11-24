/// An ID that identifies the operations within the same client. Used for notification of progress or to be canceled.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OperationId(u64);

impl OperationId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn span(self) -> tracing::Span {
        tracing::info_span!(target: "keyringd_context", "fido", op_id = self.0)
    }
}
