#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SshEllipticCurve {
    NistP256,
}

impl SshEllipticCurve {
    pub fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::NistP256 => b"nistp256",
        }
    }

    pub(crate) fn from_bytes(value: &[u8]) -> Option<Self> {
        match value {
            b"nistp256" => Some(Self::NistP256),
            _ => None,
        }
    }
}
