#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PasskeyNotFoundChoice {
    #[default]
    TryAnotherDevice,
    CreateNewPasskey,
}
