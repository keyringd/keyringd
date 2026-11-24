#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PinPromptCommand {
    Submit(String),
    Cancel,
    Dismiss,
}
