pub(crate) struct ReportSection {
    pub(crate) id: usize,
    pub(crate) crate_name: String,
    pub(crate) module_path: String,
    pub(crate) group_title: Option<String>,
    pub(crate) case_indices: Vec<usize>,
}
