use crate::execution::ActiveStep;

pub(super) enum CaseExecutionState {
    Ready,
    RunningStep(ActiveStep),
    Skipped { reason: String },
}
