use serde::{Deserialize, Serialize};

use super::{Evidence, SourceLocation, StepKind, StepOutcome};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct StepResult {
    pub kind: StepKind,
    pub description: String,
    pub source: SourceLocation,
    pub outcome: StepOutcome,
    pub duration_ms: u128,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
}
