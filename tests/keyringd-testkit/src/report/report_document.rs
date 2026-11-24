use std::collections::BTreeMap;
use std::path::Path;

use crate::model::{CaseOutcome, CaseResult};
use crate::report::report_path::{display_module_path, portable_path};

pub(crate) struct ReportDocument {
    pub(crate) results: Vec<CaseResult>,
    pub(crate) sections: Vec<ReportSection>,
    pub(crate) table_of_contents: TocNode,
    pub(crate) passed: usize,
    pub(crate) failed: usize,
    pub(crate) skipped: usize,
}

use super::ReportSection;

use super::TocNode;

use super::TocGroup;

use super::TocCase;

use super::SectionKey;

impl ReportDocument {
    pub(crate) fn new(mut results: Vec<CaseResult>, project_root: Option<&Path>) -> Self {
        results.sort_by_cached_key(|result| {
            (
                portable_path(&result.source.file, project_root),
                result.source.line,
                result.source.column,
                result.id.clone(),
            )
        });

        let mut section_ids = BTreeMap::<SectionKey, usize>::new();
        let mut sections = Vec::<ReportSection>::new();
        let mut section_for_case = vec![0; results.len()];
        for (index, result) in results.iter().enumerate() {
            let key = SectionKey {
                source_file: portable_path(&result.source.file, project_root),
                crate_name: result.crate_name.clone(),
                module_path: display_module_path(result).to_owned(),
                group_title: result.group.clone(),
            };
            let section_id = if let Some(section_id) = section_ids.get(&key) {
                *section_id
            } else {
                let section_id = sections.len();
                sections.push(ReportSection {
                    id: section_id,
                    crate_name: key.crate_name.clone(),
                    module_path: key.module_path.clone(),
                    group_title: key.group_title.clone(),
                    case_indices: Vec::new(),
                });
                section_ids.insert(key, section_id);
                section_id
            };
            sections[section_id].case_indices.push(index);
            section_for_case[index] = section_id;
        }

        let table_of_contents =
            build_table_of_contents(&results, &sections, &section_for_case, project_root);
        let section_positions = table_of_contents
            .section_ids_in_display_order()
            .into_iter()
            .enumerate()
            .map(|(position, section_id)| (section_id, position))
            .collect::<BTreeMap<_, _>>();
        sections.sort_by_key(|section| {
            section_positions
                .get(&section.id)
                .copied()
                .expect("every report section must appear in the table of contents")
        });
        let passed = count(&results, CaseOutcome::Passed);
        let failed = count(&results, CaseOutcome::Failed);
        let skipped = count(&results, CaseOutcome::Skipped);

        Self {
            results,
            sections,
            table_of_contents,
            passed,
            failed,
            skipped,
        }
    }
}

fn build_table_of_contents(
    results: &[CaseResult],
    sections: &[ReportSection],
    section_for_case: &[usize],
    project_root: Option<&Path>,
) -> TocNode {
    let mut root = TocNode::default();
    for (case_index, result) in results.iter().enumerate() {
        let path = portable_path(&result.source.file, project_root);
        let mut components = path.split('/').peekable();
        let mut node = &mut root;
        while let Some(component) = components.next() {
            if components.peek().is_some() {
                node = node.directories.entry(component.to_owned()).or_default();
                continue;
            }
            let section_id = section_for_case[case_index];
            let file = node.files.entry(component.to_owned()).or_default();
            file.first_section.get_or_insert(section_id);
            if !file.sections.contains(&section_id) {
                file.sections.push(section_id);
            }
            let toc_case = TocCase {
                title: result.title.clone(),
                case_index,
            };
            if let Some(group_title) = &sections[section_id].group_title {
                if let Some(group) = file
                    .groups
                    .iter_mut()
                    .find(|group| group.section_id == section_id)
                {
                    group.cases.push(toc_case);
                } else {
                    file.groups.push(TocGroup {
                        title: group_title.clone(),
                        section_id,
                        cases: vec![toc_case],
                    });
                }
            } else {
                file.cases.push(toc_case);
            }
        }
    }
    root
}

fn count(results: &[CaseResult], outcome: CaseOutcome) -> usize {
    results
        .iter()
        .filter(|result| result.outcome == outcome)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SourceLocation;

    keyringd_testkit::scenario! {
        group "ReportDocument::new" {
            case "Uses the table of contents file order for body sections" {
                verify "keep result sections in the same file order as the contents tree" {
                    std::thread::spawn(|| {
                        let document = ReportDocument::new(
                            vec![
                                result("z-module", "crate/src/z.rs", CaseOutcome::Failed),
                                result("a-module", "crate/src/a.rs", CaseOutcome::Passed),
                            ],
                            None,
                        );

                        assert_eq!(document.sections[0].module_path, "a-module");
                        assert_eq!(document.sections[1].module_path, "z-module");
                        let source_directory = &document.table_of_contents.directories["crate"].directories["src"];
                        assert_eq!(source_directory.files["a.rs"].first_section, Some(0));
                        assert_eq!(source_directory.files["z.rs"].first_section, Some(1));
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ReportDocument::new" {
            case "Uses directory first tree order for body sections" {
                verify "show directory subtrees before sibling files in the report body" {
                    std::thread::spawn(|| {
                        let document = ReportDocument::new(
                            vec![
                                result("root-module", "crate/src/agent.rs", CaseOutcome::Passed),
                                result(
                                    "nested-module",
                                    "crate/src/tests/identity_listing.rs",
                                    CaseOutcome::Passed,
                                ),
                            ],
                            None,
                        );

                        assert_eq!(document.sections[0].module_path, "nested-module");
                        assert_eq!(document.sections[1].module_path, "root-module");
                        let source_directory = &document.table_of_contents.directories["crate"].directories["src"];
                        assert_eq!(
                            source_directory.directories["tests"].files["identity_listing.rs"].first_section,
                            Some(document.sections[0].id)
                        );
                        assert_eq!(
                            source_directory.files["agent.rs"].first_section,
                            Some(document.sections[1].id)
                        );
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ReportDocument::new" {
            case "Orders groups and cases by their source locations" {
                verify "sort case groups and their entries by the recorded source positions" {
                    std::thread::spawn(|| {
                        let mut later_group_first_case =
                            result("module", "crate/src/cases.rs", CaseOutcome::Passed);
                        later_group_first_case.id = "later-group-first".to_owned();
                        later_group_first_case.title = "alphabetically last".to_owned();
                        later_group_first_case.group = Some("Z method".to_owned());
                        later_group_first_case.source = SourceLocation::new("crate/src/cases.rs", 10, 1);

                        let mut earlier_group_failed_case =
                            result("module", "crate/src/cases.rs", CaseOutcome::Failed);
                        earlier_group_failed_case.id = "later-group-second".to_owned();
                        earlier_group_failed_case.title = "alphabetically first".to_owned();
                        earlier_group_failed_case.group = Some("Z method".to_owned());
                        earlier_group_failed_case.source = SourceLocation::new("crate/src/cases.rs", 20, 1);

                        let mut alphabetically_first_group =
                            result("module", "crate/src/cases.rs", CaseOutcome::Passed);
                        alphabetically_first_group.id = "alphabetically-first-group".to_owned();
                        alphabetically_first_group.group = Some("A method".to_owned());
                        alphabetically_first_group.source = SourceLocation::new("crate/src/cases.rs", 30, 1);

                        let document = ReportDocument::new(
                            vec![
                                alphabetically_first_group,
                                earlier_group_failed_case,
                                later_group_first_case,
                            ],
                            None,
                        );

                        assert_eq!(
                            document
                                .results
                                .iter()
                                .map(|result| result.source.line)
                                .collect::<Vec<_>>(),
                            [10, 20, 30]
                        );
                        assert_eq!(
                            document.sections[0].group_title.as_deref(),
                            Some("Z method")
                        );
                        assert_eq!(document.sections[0].case_indices, [0, 1]);
                        assert_eq!(
                            document.sections[1].group_title.as_deref(),
                            Some("A method")
                        );
                        let source_directory = &document.table_of_contents.directories["crate"].directories["src"];
                        let groups = &source_directory.files["cases.rs"].groups;
                        assert_eq!(groups.len(), 2);
                        assert_eq!(groups[0].title, "Z method");
                        assert_eq!(groups[0].section_id, 0);
                        assert_eq!(
                            groups[0].cases,
                            [
                                TocCase {
                                    title: "alphabetically last".to_owned(),
                                    case_index: 0,
                                },
                                TocCase {
                                    title: "alphabetically first".to_owned(),
                                    case_index: 1,
                                },
                            ]
                        );
                        assert_eq!(groups[1].title, "A method");
                        assert_eq!(groups[1].section_id, 1);
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    fn result(module_path: &str, source_file: &str, outcome: CaseOutcome) -> CaseResult {
        CaseResult {
            id: format!("crate::{module_path}"),
            title: module_path.to_owned(),
            rationale: None,
            specifications: Vec::new(),
            source_code: None,
            source_error: None,
            crate_name: "crate".to_owned(),
            module_path: module_path.to_owned(),
            group: None,
            source: SourceLocation::new(source_file, 1, 1),
            outcome,
            duration_ms: 0,
            failure: None,
            skipped_reason: None,
            steps: Vec::new(),
            evidence: Vec::new(),
        }
    }
}
