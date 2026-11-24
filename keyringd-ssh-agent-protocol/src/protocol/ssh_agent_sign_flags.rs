#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SshAgentSignFlags(u32);

impl SshAgentSignFlags {
    pub const RSA_SHA2_256: Self = Self(2);
    pub const RSA_SHA2_512: Self = Self(4);

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, flags: Self) -> bool {
        self.0 & flags.0 == flags.0
    }

    pub const fn unknown_bits(self) -> u32 {
        self.0 & !(Self::RSA_SHA2_256.0 | Self::RSA_SHA2_512.0)
    }
}
