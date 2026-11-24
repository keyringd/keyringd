#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeviceKeyAvailabilityError {
    Unavailable(String),
    AccessDenied { message: String, can_grant: bool },
}

impl DeviceKeyAvailabilityError {
    pub fn message(&self) -> &str {
        match self {
            Self::Unavailable(message) | Self::AccessDenied { message, .. } => message,
        }
    }
}
