use super::InteractionId;

/// A request to enter a new PIN to be set in the authenticator. Conveys the minimum number of characters and the information about the refusal to be set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NewPinInteraction {
    id: InteractionId,
    minimum_length: u64,
    rejected_by_authenticator: bool,
}
impl NewPinInteraction {
    pub(crate) fn new(
        id: InteractionId,
        minimum_length: u64,
        rejected_by_authenticator: bool,
    ) -> Self {
        Self {
            id,
            minimum_length,
            rejected_by_authenticator,
        }
    }
    pub const fn id(&self) -> InteractionId {
        self.id
    }
    pub const fn minimum_length(&self) -> u64 {
        self.minimum_length
    }
    pub const fn rejected_by_authenticator(&self) -> bool {
        self.rejected_by_authenticator
    }
}
