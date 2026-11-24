use super::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Requirement {
    pub(super) source: SpecificationSource,
    pub(super) text: String,
    #[serde(default)]
    pub(super) tests: Vec<String>,
    #[serde(default)]
    pub(super) scope: RequirementScope,
    #[serde(default)]
    pub(super) reason: Option<String>,
}
