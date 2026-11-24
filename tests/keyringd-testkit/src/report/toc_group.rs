use super::*;

pub(crate) struct TocGroup {
    pub(crate) title: String,
    pub(crate) section_id: usize,
    pub(crate) cases: Vec<TocCase>,
}
