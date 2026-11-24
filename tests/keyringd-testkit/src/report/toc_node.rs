use std::collections::BTreeMap;

use super::*;

#[derive(Default)]
pub(crate) struct TocNode {
    pub(crate) directories: BTreeMap<String, TocNode>,
    pub(crate) files: BTreeMap<String, TocFile>,
}

impl TocNode {
    pub(super) fn section_ids_in_display_order(&self) -> Vec<usize> {
        let mut section_ids = Vec::new();
        self.collect_section_ids(&mut section_ids);
        section_ids
    }

    pub(super) fn collect_section_ids(&self, section_ids: &mut Vec<usize>) {
        for directory in self.directories.values() {
            directory.collect_section_ids(section_ids);
        }
        for file in self.files.values() {
            section_ids.extend(file.sections.iter().copied());
        }
    }
}
