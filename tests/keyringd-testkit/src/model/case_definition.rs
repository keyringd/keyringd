use super::SourceLocation;

#[derive(Clone)]
pub(crate) struct CaseDefinition {
    pub(crate) id: &'static str,
    pub(crate) title: &'static str,
    pub(crate) rationale: Option<&'static str>,
    pub(crate) specifications: &'static [&'static str],
    pub(crate) crate_name: &'static str,
    pub(crate) module_path: &'static str,
    pub(crate) group: Option<&'static str>,
    pub(crate) source: SourceLocation,
}
