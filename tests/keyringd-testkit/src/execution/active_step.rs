use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::*;
use crate::model::{Evidence, SourceLocation, StepKind, StepOutcome};

pub(crate) struct ActiveStep {
    pub(super) kind: StepKind,
    pub(super) description: &'static str,
    pub(super) source: SourceLocation,
    pub(super) started_at: Instant,
    pub(super) evidence: Arc<Mutex<EvidenceCollector>>,
}

impl ActiveStep {
    pub(crate) fn start(kind: StepKind, description: &'static str, source: SourceLocation) -> Self {
        let evidence = Arc::new(Mutex::new(EvidenceCollector::default()));
        activate_step(&evidence);
        Self {
            kind,
            description,
            source,
            started_at: Instant::now(),
            evidence,
        }
    }

    pub(crate) fn attach(&mut self, evidence: Evidence) {
        self.evidence
            .lock()
            .expect("step evidence lock poisoned")
            .evidence
            .push(evidence);
    }

    pub(crate) fn finish(self, outcome: StepOutcome) -> RecordedStep {
        let evidence = {
            let mut collector = self
                .evidence
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            collector.closed = true;
            std::mem::take(&mut collector.evidence)
        };
        deactivate_step(&self.evidence);
        RecordedStep {
            kind: self.kind,
            description: self.description,
            source: self.source.clone(),
            duration: self.started_at.elapsed(),
            outcome,
            evidence,
        }
    }

    pub(crate) fn description(&self) -> &'static str {
        self.description
    }
}

impl Drop for ActiveStep {
    fn drop(&mut self) {
        self.evidence
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .closed = true;
        deactivate_step(&self.evidence);
    }
}
