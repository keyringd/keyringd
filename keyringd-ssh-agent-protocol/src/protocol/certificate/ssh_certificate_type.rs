#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SshCertificateType {
    User,
    Host,
}

impl SshCertificateType {
    pub(crate) fn from_wire(value: u32) -> Option<Self> {
        match value {
            1 => Some(Self::User),
            2 => Some(Self::Host),
            _ => None,
        }
    }

    pub(crate) fn to_wire(self) -> u32 {
        match self {
            Self::User => 1,
            Self::Host => 2,
        }
    }
}
