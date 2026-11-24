/// The version of the cryptographic protocol used for PIN/UV authentication.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PinUvAuthProtocol {
    One,
    Two,
}

impl PinUvAuthProtocol {
    pub const fn version(self) -> u64 {
        match self {
            Self::One => 1,
            Self::Two => 2,
        }
    }
}
