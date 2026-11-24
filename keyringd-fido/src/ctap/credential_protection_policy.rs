#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum CredentialProtectionPolicy {
    UserVerificationOptional = 1,
    UserVerificationOptionalWithCredentialIdList = 2,
    UserVerificationRequired = 3,
}

impl CredentialProtectionPolicy {
    pub const fn value(self) -> u8 {
        self as u8
    }
}
