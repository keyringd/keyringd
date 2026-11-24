#[derive(Clone, Debug, PartialEq)]
pub enum ToastMessage {
    Success(String),
    Error(String),
}

impl ToastMessage {
    pub fn text(&self) -> &str {
        match self {
            Self::Success(text) | Self::Error(text) => text,
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}
