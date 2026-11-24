use serde::{Deserialize, Serialize};

use super::{CaseOutcome, Evidence, SourceLocation};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct CaseResult {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(default)]
    pub specifications: Vec<String>,
    pub source_code: Option<String>,
    pub source_error: Option<String>,
    pub crate_name: String,
    pub module_path: String,
    #[serde(default)]
    pub group: Option<String>,
    pub source: SourceLocation,
    pub outcome: CaseOutcome,
    pub duration_ms: u128,
    pub failure: Option<String>,
    pub skipped_reason: Option<String>,
    pub steps: Vec<StepResult>,
    pub evidence: Vec<Evidence>,
}

use super::StepResult;
