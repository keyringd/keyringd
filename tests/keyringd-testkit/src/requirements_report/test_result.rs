use super::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct TestResult {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) source: TestSource,
    pub(super) outcome: TestOutcome,
    pub(super) duration_ms: u128,
    #[serde(default)]
    pub(super) failure: Option<String>,
    #[serde(default)]
    pub(super) skipped_reason: Option<String>,
}
