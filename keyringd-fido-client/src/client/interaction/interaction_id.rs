use crate::OperationId;

/// Identify a single interaction request with an operation ID and a sequence number in the operation. Use the response to match the original request.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct InteractionId {
    operation_id: OperationId,
    sequence: u64,
}

impl InteractionId {
    pub const fn new(operation_id: OperationId, sequence: u64) -> Self {
        Self {
            operation_id,
            sequence,
        }
    }

    pub const fn operation_id(self) -> OperationId {
        self.operation_id
    }
    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub fn span(self) -> tracing::Span {
        tracing::info_span!(target: "keyringd_context", "interaction", ix_id = %self)
    }

    pub async fn scope<F: std::future::Future>(self, future: F) -> F::Output {
        use tracing::Instrument;
        future.instrument(self.span()).await
    }
}

impl std::fmt::Display for InteractionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.operation_id.get(), self.sequence)
    }
}
