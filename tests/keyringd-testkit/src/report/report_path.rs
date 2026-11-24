use std::path::Path;

use crate::model::CaseResult;

pub(crate) fn display_module_path(result: &CaseResult) -> &str {
    if result.group.is_some() {
        result
            .module_path
            .rsplit_once("::")
            .map_or(result.module_path.as_str(), |(parent, _)| parent)
    } else {
        &result.module_path
    }
}

pub(crate) fn portable_path(file: &str, project_root: Option<&Path>) -> String {
    let path = file.replace('\\', "/");
    project_root
        .and_then(|root| strip_portable_prefix(&path, &portable(root)))
        .unwrap_or(path)
}

/// Resolves an evidence path into a link the report document can follow.
///
/// Evidence is normally written inside the report directory and recorded
/// relative to it, which keeps the directory valid wherever it is copied. An
/// absolute path is resolved the same way instead of being left pointing at a
/// location that does not exist next to the document.
pub(crate) fn evidence_href(
    path: &str,
    report_directory: &Path,
    project_root: Option<&Path>,
    project_base_href: Option<&str>,
) -> String {
    let path = path.replace('\\', "/");
    if !is_portably_absolute(&path) {
        return path;
    }
    if let Some(inside_report) = strip_portable_prefix(&path, &portable(report_directory)) {
        return inside_report;
    }
    match project_root.and_then(|root| strip_portable_prefix(&path, &portable(root))) {
        Some(inside_project) => {
            format!("{}{inside_project}", project_base_href.unwrap_or_default())
        }
        None => path,
    }
}

pub(crate) fn source_url(
    file: &str,
    line: u32,
    project_root: Option<&Path>,
    project_base_href: Option<&str>,
    source_base_url: Option<&str>,
) -> String {
    let source_path = portable_path(file, project_root);
    match source_base_url {
        Some(base_url) => format!(
            "{}/{}#L{line}",
            base_url.trim_end_matches('/'),
            source_path.trim_start_matches('/')
        ),
        None => format!(
            "{}{source_path}#L{line}",
            project_base_href.unwrap_or_default()
        ),
    }
}

pub(crate) fn project_base_href(report_directory: &Path, project_root: &Path) -> Option<String> {
    let report_directory = portable(report_directory);
    let report_directory = if is_portably_absolute(&report_directory) {
        report_directory
    } else {
        portable(&std::env::current_dir().ok()?.join(report_directory))
    };
    let relative_report_directory =
        strip_portable_prefix(&report_directory, &portable(project_root))?;
    let depth = relative_report_directory
        .split('/')
        .filter(|component| !component.is_empty())
        .count();
    Some(if depth == 0 {
        "./".to_owned()
    } else {
        "../".repeat(depth)
    })
}

fn portable(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn strip_portable_prefix(path: &str, prefix: &str) -> Option<String> {
    let prefix = prefix.trim_end_matches('/');
    if path == prefix {
        return Some(String::new());
    }
    path.strip_prefix(prefix)
        .and_then(|suffix| suffix.strip_prefix('/'))
        .map(ToOwned::to_owned)
}

fn is_portably_absolute(path: &str) -> bool {
    path.starts_with('/')
        || path
            .as_bytes()
            .get(1)
            .is_some_and(|character| *character == b':')
}
