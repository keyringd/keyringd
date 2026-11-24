#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustEntryKind {
    CertificateAuthority,
    DeviceKey,
    Passkey,
    DirectKeys,
}

impl TrustEntryKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::CertificateAuthority => "Certificate Authority Public Key",
            Self::DeviceKey => "Device Key Public Key",
            Self::Passkey => "Passkey Public Key",
            Self::DirectKeys => "Device + Passkey Public Keys",
        }
    }
    pub fn copy_label(self) -> &'static str {
        match self {
            Self::CertificateAuthority => "Copy Certificate Authority Public Key",
            Self::DeviceKey => "Copy Device Key Public Key",
            Self::Passkey => "Copy Passkey Public Key",
            Self::DirectKeys => "Copy Device + Passkey Public Keys",
        }
    }
    pub fn success_message(self) -> &'static str {
        match self {
            Self::CertificateAuthority => "Certificate Authority Public Key copied",
            Self::DeviceKey => "Device Key Public Key copied",
            Self::Passkey => "Passkey Public Key copied",
            Self::DirectKeys => "Device and Passkey Public Keys copied",
        }
    }
}
