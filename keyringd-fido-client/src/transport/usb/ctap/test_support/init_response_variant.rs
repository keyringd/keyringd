/// Types of responses to be generated in the test, such as mismatched initialization responses and incorrect values and future protocol versions.
#[derive(Clone, Copy)]
pub(crate) enum InitResponseVariant {
    NonceMismatch,
    InvalidChannel(u32),
    MissingCborCapability,
    FutureProtocolVersion(u8),
}
