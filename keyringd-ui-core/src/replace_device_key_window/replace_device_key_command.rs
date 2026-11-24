#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplaceDeviceKeyCommand {
    Replace,
    OpenTrustEntriesAndClose,
    Close,
}
