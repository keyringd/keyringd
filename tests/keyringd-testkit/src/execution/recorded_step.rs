use std::time::Duration;

use crate::model::{Evidence, SourceLocation, StepKind, StepOutcome, StepResult};

pub(crate) struct RecordedStep {
    pub(super) kind: StepKind,
    pub(super) description: &'static str,
    pub(super) source: SourceLocation,
    pub(super) duration: Duration,
    pub(super) outcome: StepOutcome,
    pub(super) evidence: Vec<Evidence>,
}

impl From<&RecordedStep> for StepResult {
    fn from(step: &RecordedStep) -> Self {
        Self {
            kind: step.kind,
            description: step.description.to_owned(),
            source: step.source.clone(),
            outcome: step.outcome,
            duration_ms: step.duration.as_millis(),
            evidence: step.evidence.clone(),
        }
    }
}
