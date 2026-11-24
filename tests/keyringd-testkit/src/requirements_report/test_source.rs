use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct TestSource {
    pub(super) file: String,
    pub(super) line: u32,
}
