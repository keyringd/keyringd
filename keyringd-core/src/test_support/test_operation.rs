#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TestOperation {
    ProfilesSaved(Vec<Vec<u8>>),
    DeviceKeyDeleted(Vec<u8>),
}
