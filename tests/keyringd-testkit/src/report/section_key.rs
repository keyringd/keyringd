#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct SectionKey {
    pub(super) source_file: String,
    pub(super) crate_name: String,
    pub(super) module_path: String,
    pub(super) group_title: Option<String>,
}
