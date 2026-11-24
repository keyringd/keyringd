use super::*;

#[derive(Default)]
pub(crate) struct TocFile {
    pub(crate) first_section: Option<usize>,
    pub(crate) sections: Vec<usize>,
    pub(crate) groups: Vec<TocGroup>,
    pub(crate) cases: Vec<TocCase>,
}
