#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceKeyGenerationReason {
    InitialSetup,
    PasswordProtectionChange,
    Replacement,
}
