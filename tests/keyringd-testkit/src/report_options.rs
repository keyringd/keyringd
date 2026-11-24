#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportOptions {
    pub source_base_url: Option<String>,
    pub include_requirements: bool,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            source_base_url: None,
            include_requirements: true,
        }
    }
}
