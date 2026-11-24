#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileSetupScreen {
    Form,
    PasskeyNotFound,
    Credentials,
    Interaction,
    Completed,
    Cancelling,
}
