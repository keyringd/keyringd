use super::InteractionId;

/// A request to enter the PIN that has already been set in the authenticator for identity verification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PinInteraction {
    id: InteractionId,
    attempts_remaining: Option<u64>,
}
impl PinInteraction {
    pub(crate) fn new(id: InteractionId, attempts_remaining: Option<u64>) -> Self {
        Self {
            id,
            attempts_remaining,
        }
    }
    pub const fn id(&self) -> InteractionId {
        self.id
    }
    pub const fn attempts_remaining(&self) -> Option<u64> {
        self.attempts_remaining
    }
}
