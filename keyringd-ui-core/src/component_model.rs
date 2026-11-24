/// Updates UI state synchronously and returns an optional instruction for the host.
/// The OS-specific command handler performs the requested work outside this transition.
pub trait ComponentModel {
    type Event;
    type Command;
    fn update(&mut self, event: Self::Event) -> Option<Self::Command>;
}
