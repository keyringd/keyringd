use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SpecificationSource {
    pub(super) title: String,
    pub(super) url: String,
}
