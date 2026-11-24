use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FixtureRecord {
    pub directory: PathBuf,
}
