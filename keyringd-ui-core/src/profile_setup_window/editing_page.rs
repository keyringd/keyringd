use super::PasskeyNotFoundChoice;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditingPage {
    Form,
    PasskeyNotFound(PasskeyNotFoundChoice),
}
