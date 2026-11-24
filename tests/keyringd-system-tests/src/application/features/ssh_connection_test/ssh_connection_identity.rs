#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SshConnectionIdentity {
    DeviceCertificate,
    DeviceKey,
    Passkey,
}

impl SshConnectionIdentity {
    pub fn choice_name(self) -> &'static str {
        match self {
            Self::DeviceCertificate => "Device Certificate (Recommended)",
            Self::DeviceKey => "Device Key (Legacy-compatible)",
            Self::Passkey => "Passkey (Legacy-compatible)",
        }
    }

    pub fn result_name(self) -> &'static str {
        match self {
            Self::DeviceCertificate => "Device Certificate",
            Self::DeviceKey => "Device Key",
            Self::Passkey => "Passkey",
        }
    }

    pub fn identifier(self) -> &'static str {
        match self {
            Self::DeviceCertificate => "connection-test.identity.device-certificate",
            Self::DeviceKey => "connection-test.identity.device-key",
            Self::Passkey => "connection-test.identity.passkey",
        }
    }
}
