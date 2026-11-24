#[derive(Debug, Eq, PartialEq)]
pub enum ProfileDetailsCommand {
    OpenTrustEntries,
    OpenReplaceDeviceKey,
    Close,
}
