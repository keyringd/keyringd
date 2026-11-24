use crate::model::SourceLocation;

pub struct CaseMetadata {
    pub id: &'static str,
    pub title: &'static str,
    pub rationale: Option<&'static str>,
    pub specifications: &'static [&'static str],
    pub source_end_line: u32,
    pub crate_name: &'static str,
    pub module_path: &'static str,
    pub group: Option<&'static str>,
    pub source: SourceLocation,
}
