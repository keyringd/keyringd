use super::InteractionId;

/// A request for a caller to choose which one to use, either USB or Hybrid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransportInteraction {
    id: InteractionId,
}
impl TransportInteraction {
    pub(crate) fn new(id: InteractionId) -> Self {
        Self { id }
    }
    pub const fn id(&self) -> InteractionId {
        self.id
    }
}
