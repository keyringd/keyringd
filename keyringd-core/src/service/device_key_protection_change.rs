use crate::model::device_key::LocalAuthentication;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceKeyProtectionChange {
    Enable,
    ChangePassword,
    Disable,
}

impl DeviceKeyProtectionChange {
    /// The local authentication setting required before applying this change.
    pub fn expected_current_authentication(self) -> LocalAuthentication {
        match self {
            Self::Enable => LocalAuthentication::NotRequired,
            Self::ChangePassword | Self::Disable => LocalAuthentication::Required,
        }
    }

    pub fn requested(self) -> LocalAuthentication {
        match self {
            Self::Enable | Self::ChangePassword => LocalAuthentication::Required,
            Self::Disable => LocalAuthentication::NotRequired,
        }
    }
}
