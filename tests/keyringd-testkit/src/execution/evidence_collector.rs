use crate::model::Evidence;

/// A step closes its collector before its evidence is consumed.
#[derive(Default)]
pub(crate) struct EvidenceCollector {
    pub(crate) closed: bool,
    pub(crate) evidence: Vec<Evidence>,
}
