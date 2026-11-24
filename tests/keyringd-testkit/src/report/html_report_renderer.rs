use std::path::Path;

use maud::{DOCTYPE, PreEscaped, html};

use super::{ReportDocument, TocNode};
use crate::model::{CaseOutcome, CaseResult, Evidence, StepOutcome};
use crate::report::report_path::{evidence_href, portable_path, project_base_href, source_url};

const STYLE: &str = include_str!("assets/report.css");

const SCRIPT: &str = include_str!("assets/report.js");

pub(crate) fn render_html_report(
    results: Vec<CaseResult>,
    directory: &Path,
    project_root: Option<&Path>,
    source_base_url: Option<&str>,
    include_requirements: bool,
) -> String {
    let document = ReportDocument::new(results, project_root);
    let project_base_href = project_root.and_then(|root| project_base_href(directory, root));
    render(
        &document,
        directory,
        project_root,
        project_base_href.as_deref(),
        source_base_url,
        include_requirements,
    )
}

fn metadata_value(value: &Result<String, String>) -> String {
    match value {
        Ok(value) => value.clone(),
        Err(error) => format!("Unavailable: {error}"),
    }
}

fn render(
    document: &ReportDocument,
    report_directory: &Path,
    project_root: Option<&Path>,
    project_base_href: Option<&str>,
    source_base_url: Option<&str>,
    include_requirements: bool,
) -> String {
    let metadata = crate::runner::ReportRunMetadata::load(report_directory);
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width,initial-scale=1";
                title { "keyringd test report" }
                link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.12.0/styles/github.min.css" media="(prefers-color-scheme: light)";
                link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.12.0/styles/github-dark.min.css" media="(prefers-color-scheme: dark)";
                style { (PreEscaped(STYLE)) }
            }
            body {
                div class="layout" {
                    aside class="toc" aria-label="Table of contents" {
                        div class="toc-title" { "ToC" }
                        nav class="toc-tree" { (toc_node(&document.table_of_contents, true)) }
                    }
                    main {
                        div class="summary" {
                            h1 { "Test report" }
                            span class="counts" { (document.results.len()) " cases · " (document.passed) " passed · " (document.failed) " failed · " (document.skipped) " skipped" }
                            @if include_requirements {
                                a href="requirements.html" { "Requirements" }
                            }
                            div class="translation" {
                                button id="show-evidence" class="evidence-control" type="button" aria-pressed="false" {
                                    "Evidence details"
                                }
                                span id="translation-status" class="translation-status" aria-live="polite" {}
                                label class="translation-control" for="language" {
                                    span { "Language" }
                                    select id="language" {
                                        option value="original" { "Original" }
                                    }
                                }
                            }
                        }
                        @match &metadata {
                            Ok(Some(run)) => {
                                details class="run-metadata" {
                                    summary { "Execution environment" }
                                    table {
                                        tr { th { "Run" } td { (&run.run_id) } }
                                        tr { th { "Started (UTC)" } td { (metadata_value(&run.started_at_utc)) } }
                                        tr { th { "OS" } td { (&run.operating_system) pre { (metadata_value(&run.os_version)) } } }
                                        @if let Some(environment) = &run.test_environment {
                                            tr { th { "Test environment" } td { pre { (metadata_value(environment)) } } }
                                        }
                                        tr { th { "Source revision" } td { (metadata_value(&run.source_revision)) } }
                                        tr { th { "Uncommitted changes" } td { @match &run.source_dirty { Ok(dirty) => (dirty), Err(error) => { "Unavailable: " (error) } } } }
                                        @for tool in &run.tools {
                                            tr { th { (&tool.name) } td {
                                                @if let Some(path) = &tool.path { code { (path.display()) } }
                                                pre { (metadata_value(&tool.version)) }
                                            } }
                                        }
                                    }
                                    a href="run-metadata.json" { "Execution metadata (JSON)" }
                                }
                            }
                            Err(error) => { p { "Execution metadata could not be read: " (error) } }
                            Ok(None) => {}
                        }
                        div class="toolbar" {
                            input id="search" type="search" placeholder="Filter cases and steps";
                            select id="status" {
                                option value="all" { "All results" }
                                option value="fail" { "Failed" }
                                option value="pass" { "Passed" }
                                option value="skip" { "Skipped" }
                            }
                        }
                        @for section in &document.sections {
                            section id=(format!("group-{}", section.id)) class="group" {
                                div class="group-title" {
                                    (&section.crate_name) " · " (&section.module_path)
                                    @if let Some(group_title) = &section.group_title {
                                        " · " (group_title)
                                    }
                                }
                                @for &index in &section.case_indices {
                                    @let report = &document.results[index];
                                    (case_report(report, index, report_directory, project_root, project_base_href, source_base_url))
                                }
                            }
                        }
                    }
                }
                script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.12.0/highlight.min.js" {}
                script { (PreEscaped(SCRIPT)) }
            }
        }
    }
    .into_string()
}

fn case_report(
    report: &CaseResult,
    index: usize,
    report_directory: &Path,
    project_root: Option<&Path>,
    project_base_href: Option<&str>,
    source_base_url: Option<&str>,
) -> maud::Markup {
    let status = case_outcome_label(report.outcome);
    let status_class = case_outcome_css_class(report.outcome);
    let searchable = format!(
        "{} {} {} {} {} {}",
        report.title,
        report.module_path,
        report.group.as_deref().unwrap_or_default(),
        report.rationale.as_deref().unwrap_or_default(),
        report.specifications.join(" "),
        report
            .steps
            .iter()
            .map(|step| step.description.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    )
    .to_lowercase();
    let source_path = format!(
        "{}:{}",
        portable_path(&report.source.file, project_root),
        report.source.line
    );
    let source_id = format!("case-{index}-source");
    let has_source = report.source_code.is_some();
    let mut print_specifications = Vec::new();
    for url in &report.specifications {
        let label = specification_label(url);
        if !print_specifications.contains(&label) {
            print_specifications.push(label);
        }
    }
    html! {
        article id=(format!("case-{index}")) class="case" data-status=(status_class) data-search=(searchable) {
            header class="case-header" {
                span class="case-title" {
                    span class="case-name" data-translatable { (report.title) }
                    (copy_button("copy-case-title", &report.title, "Copy case title"))
                }
                span class="meta" { (report.steps.len()) " steps · " (report.duration_ms) " ms" }
                span class={ "status " (status_class) } { (status) }
            }
            @if let Some(rationale) = &report.rationale {
                div class="case-rationale" {
                    span class="case-rationale-label" { "Why" }
                    span data-translatable { (rationale) }
                }
            }
            @if !report.specifications.is_empty() {
                div class="case-specifications" {
                    span class="case-specifications-label" { "Spec" }
                    @for url in &report.specifications {
                        a href=(url) target="_blank" rel="noopener noreferrer" title=(url) { (url) }
                    }
                    span class="print-specifications" { (print_specifications.join(" · ")) }
                }
            }
            @if let Some(reason) = &report.skipped_reason {
                div class="skip-reason" { (reason) }
            }
            @if let Some(failure) = &report.failure {
                div class="failure" { (failure) }
            }
            @if !report.steps.is_empty() {
                table class="steps" {
                    thead { tr { th { "Type" } th { "Step" } th class="time" { "Duration" } th class="step-result" { "Result" } } }
                    @for step in &report.steps {
                        tbody class="step-block" {
                            @if has_source {
                                tr class="step-row" tabindex="0" role="button" aria-controls=(&source_id) aria-expanded="false" {
                                    (step_cells(step, project_root, project_base_href, source_base_url))
                                }
                            } @else {
                                tr {
                                    (step_cells(step, project_root, project_base_href, source_base_url))
                                }
                            }
                            @if !step.evidence.is_empty() {
                                tr class="step-evidence-row" {
                                    td colspan="4" {
                                        div class="step-evidence" {
                                            @for evidence in &step.evidence {
                                                (evidence_report(evidence, report_directory, project_root, project_base_href))
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            @if let Some(source_code) = &report.source_code {
                details id=(&source_id) class="case-code" {
                    summary {
                        span { "Test source" }
                        a class="case-code-source" href=(source_url(&report.source.file, report.source.line, project_root, project_base_href, source_base_url)) target="_blank" rel="noopener noreferrer" title={ "Open test source in a new tab: " (&source_path) } aria-label="Open test source in a new tab" {
                            svg viewBox="0 0 24 24" aria-hidden="true" {
                                path d="M15 3h6v6M21 3l-9 9M10 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-5" {}
                            }
                        }
                        (copy_button("copy-source-path", &source_path, "Copy source path"))
                    }
                    pre { code class="language-rust" { (source_code) } }
                }
            } @else if let Some(source_error) = &report.source_error {
                div class="case-code source-error" { "Test source unavailable: " (source_error) }
            }
            @if !report.evidence.is_empty() {
                div class="attachments" {
                    @for evidence in &report.evidence {
                        (evidence_report(evidence, report_directory, project_root, project_base_href))
                    }
                }
            }
        }
    }
}

fn specification_label(url: &str) -> String {
    let label = if url.starts_with("https://fidoalliance.org/specs/fido-v2.2-") {
        "CTAP 2.2"
    } else if url.starts_with("https://fidoalliance.org/specs/fido-v2.3-") {
        "CTAP 2.3"
    } else if url.starts_with("https://www.w3.org/TR/webauthn-3/") {
        "WebAuthn Level 3"
    } else if url.starts_with("https://www.rfc-editor.org/rfc/rfc9987.html") {
        "RFC 9987"
    } else {
        url.split('/').nth(2).unwrap_or("Specification")
    };
    match url.split_once('#') {
        Some((_, fragment)) if !fragment.is_empty() => format!("{label} #{fragment}"),
        _ => label.to_owned(),
    }
}

fn step_cells(
    step: &crate::model::StepResult,
    project_root: Option<&Path>,
    project_base_href: Option<&str>,
    source_base_url: Option<&str>,
) -> maud::Markup {
    html! {
        td { (step.kind.label()) }
        td {
            span class="step-description" data-translatable { (step.description) }
            a class="step-line source-link" href=(source_url(&step.source.file, step.source.line, project_root, project_base_href, source_base_url)) title=(portable_path(&step.source.file, project_root)) { "L" (step.source.line) }
        }
        td class="time" { (step.duration_ms) " ms" }
        td class="step-result" { span class={ "status " (step_outcome_css_class(step.outcome)) } { (step_outcome_label(step.outcome)) } }
    }
}

fn copy_button(class: &str, text: &str, label: &str) -> maud::Markup {
    html! {
        button class={ "copy-button " (class) } type="button" data-copy-text=(text) data-copy-label=(label) title=(label) aria-label=(label) {
            svg viewBox="0 0 24 24" aria-hidden="true" {
                rect x="9" y="9" width="13" height="13" rx="2" {}
                path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" {}
            }
        }
    }
}

fn toc_node(node: &TocNode, open: bool) -> maud::Markup {
    html! {
        @for (name, child) in &node.directories {
            details open[open] {
                summary title=(name) { (name) }
                (toc_node(child, true))
            }
        }
        @for (name, file) in &node.files {
            @let targets = file.sections.iter().map(|index| format!("group-{index}")).collect::<Vec<_>>().join(" ");
            div class="toc-file" data-targets=(targets) {
                (toc_link(
                    name,
                    &format!("group-{}", file.first_section.expect("a file must contain a section")),
                    "toc-link toc-file-link",
                ))
                @if !file.groups.is_empty() || !file.cases.is_empty() {
                    ul class="toc-items" {
                        @for group in &file.groups {
                            li {
                                (toc_link(&group.title, &format!("group-{}", group.section_id), "toc-link toc-group"))
                                (toc_cases(&group.cases))
                            }
                        }
                        @for case in &file.cases {
                            li { (toc_link(&case.title, &format!("case-{}", case.case_index), "toc-link toc-case")) }
                        }
                    }
                }
            }
        }
    }
}

fn toc_cases(cases: &[super::TocCase]) -> maud::Markup {
    html! {
        ul class="toc-items" {
            @for case in cases {
                li { (toc_link(&case.title, &format!("case-{}", case.case_index), "toc-link toc-case")) }
            }
        }
    }
}

fn toc_link(name: &str, target: &str, class: &str) -> maud::Markup {
    html! {
        button class=(class) type="button" data-target=(target) title=(name) { (name) }
    }
}

fn case_outcome_label(outcome: CaseOutcome) -> &'static str {
    match outcome {
        CaseOutcome::Passed => "PASS",
        CaseOutcome::Failed => "FAIL",
        CaseOutcome::Skipped => "SKIP",
    }
}

fn case_outcome_css_class(outcome: CaseOutcome) -> &'static str {
    match outcome {
        CaseOutcome::Passed => "pass",
        CaseOutcome::Failed => "fail",
        CaseOutcome::Skipped => "skip",
    }
}

fn step_outcome_label(outcome: StepOutcome) -> &'static str {
    match outcome {
        StepOutcome::Passed => "PASS",
        StepOutcome::Failed => "FAIL",
    }
}

fn step_outcome_css_class(outcome: StepOutcome) -> &'static str {
    match outcome {
        StepOutcome::Passed => "pass",
        StepOutcome::Failed => "fail",
    }
}

fn evidence_report(
    evidence: &Evidence,
    report_directory: &Path,
    project_root: Option<&Path>,
    project_base_href: Option<&str>,
) -> maud::Markup {
    match evidence {
        Evidence::Image { name, path } => {
            let path = evidence_href(path, report_directory, project_root, project_base_href);
            html! {
                div class="attachment attachment-image" {
                    strong { (name) }
                    span class="meta" { "image" }
                    img src=(&path) alt=(name) loading="lazy";
                }
            }
        }
        Evidence::File { name, path } => {
            let path = evidence_href(path, report_directory, project_root, project_base_href);
            html! {
                details class="attachment evidence-details" {
                    summary { strong { (name) } span class="meta" { "file" } }
                    a class="source-link" href=(&path) { (&path) }
                }
            }
        }
        Evidence::Text { name, content } => text_evidence(name, "text", content),
        Evidence::Log { name, content } => text_evidence(name, "log", content),
        Evidence::Bytes { name, hexadecimal } => text_evidence(name, "bytes", hexadecimal),
    }
}

fn text_evidence(name: &str, kind: &str, content: &str) -> maud::Markup {
    html! {
        details class="attachment evidence-details" {
            summary { strong { (name) } span class="meta" { (kind) } }
            pre { (content) }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::model::{SourceLocation, StepKind, StepResult};

    fn report(outcome: CaseOutcome) -> CaseResult {
        CaseResult {
            id: "crate::case".to_owned(),
            title: "does something useful".to_owned(),
            rationale: Some("this behavior protects an important invariant".to_owned()),
            specifications: vec!["https://example.com/spec#requirement".to_owned()],
            source_code: Some("case: \"does something useful\"; verify \"state exists\" { let state = State::new(); }".to_owned()),
            source_error: None,
            crate_name: "example".to_owned(),
            module_path: "tests::example".to_owned(),
            group: None,
            source: SourceLocation::new("tests/example.rs", 12, 1),
            outcome,
            duration_ms: 3,
            failure: None,
            skipped_reason: None,
            steps: Vec::new(),
            evidence: Vec::new(),
        }
    }

    keyringd_testkit::scenario! {
        group "render_html_report" {
            case "Renders an aggregate report" {
                verify "render the case result, rationale, and specification links in HTML" {
                    std::thread::spawn(|| {
                    let mut case = report(CaseOutcome::Passed);
                            case.specifications.extend([
                                "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#first".into(),
                                "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#first".into(),
                                "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#second".into(),
                                "https://www.w3.org/TR/webauthn-3/#sctn-authenticator-data".into(),
                                "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.5".into(),
                            ]);
                            let html = render_html_report(
                                vec![case],
                                Path::new("target/test-reports"),
                                None,
                                None,
                                false,
                            );

                            assert!(html.contains("1 cases · 1 passed · 0 failed · 0 skipped"));
                            assert!(html.contains("does something useful"));
                            assert!(html.contains("this behavior protects an important invariant"));
                            assert!(html.contains("class=\"case-rationale\""));
                            assert!(html.contains("class=\"case-specifications\""));
                            assert!(html.contains("href=\"https://example.com/spec#requirement\""));
                            assert!(html.contains("<span class=\"print-specifications\">example.com #requirement · CTAP 2.2 #first · CTAP 2.2 #second · WebAuthn Level 3 #sctn-authenticator-data · RFC 9987 #section-5.5</span>"));
                            assert!(html.contains(".html#second\""));
                            assert!(html.contains("rel=\"noopener noreferrer\""));
                            assert!(html.contains(" open>"));
                            assert!(html.contains("aria-label=\"Table of contents\""));
                            assert!(html.contains("<div class=\"toc-title\">ToC</div>"));
                            assert!(html.contains("data-target=\"group-0\""));
                            assert!(html.contains("id=\"group-0\""));
                            assert!(html.contains("id=\"case-0\""));
                            assert!(html.contains("<article id=\"case-0\" class=\"case\""));
                            assert!(html.contains("<header class=\"case-header\">"));
                            assert!(!html.contains("<details id=\"case-0\""));
                            assert!(html.contains("<select id=\"language\">"));
                            assert!(html.contains("id=\"translation-status\""));
                            assert!(html.contains("searchParams.get('lang')"));
                            assert!(html.contains("searchParams.set('lang', targetLanguage)"));
                            assert!(html.contains("history.replaceState(null, '', url.href)"));
                            assert!(html.contains("Translation needs confirmation."));
                            assert!(html.contains("className = 'translation-retry'"));
                            assert!(html.contains("class=\"case-name\" data-translatable"));
                            assert!(html.contains("class=\"copy-button copy-case-title\""));
                            assert!(html.contains("data-copy-text=\"does something useful\""));
                            assert!(html.contains("aria-label=\"Copy case title\""));
                            assert!(html.contains(">example.rs</button>"));
                            assert!(html.contains("class=\"toc-link toc-case\""));
                            assert!(html.contains("data-target=\"case-0\""));
                            assert!(html.contains(">does something useful</button>"));
                            assert!(html.contains("highlight.js/11.12.0/highlight.min.js"));
                            assert!(!html.contains("crate::case"));
                    }).join().unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "evidence_report" {
            case "Collapses non image evidence and keeps screenshots visible" {
                verify "collapse textual evidence by default and keep the image preview visible" {
                    std::thread::spawn(|| {
                    let evidence = [
                                Evidence::Text {
                                    name: "Output".into(),
                                    content: "text <output>".into(),
                                },
                                Evidence::Log {
                                    name: "Server log".into(),
                                    content: "log output".into(),
                                },
                                Evidence::Bytes {
                                    name: "Response".into(),
                                    hexadecimal: "0102".into(),
                                },
                                Evidence::File {
                                    name: "Trace".into(),
                                    path: "trace.json".into(),
                                },
                            ];
                            for item in evidence {
                                let html =
                                    evidence_report(&item, Path::new("target/test-reports"), None, None).into_string();
                                assert!(html.starts_with("<details class=\"attachment evidence-details\"><summary>"));
                                assert!(html.ends_with("</details>"));
                                assert!(!html.contains(" open"));
                            }
                            let mut result = report(CaseOutcome::Passed);
                            result.evidence.push(Evidence::Image {
                                name: "Screenshot".into(),
                                path: "page.png".into(),
                            });
                            result.evidence.push(Evidence::Log {
                                name: "Server log".into(),
                                content: "log output".into(),
                            });
                            let html = render_html_report(
                                vec![result],
                                Path::new("target/test-reports"),
                                None,
                                None,
                                false,
                            );
                            assert!(html.contains("<button id=\"show-evidence\" class=\"evidence-control\" type=\"button\" aria-pressed=\"false\">Evidence details</button>"));
                            let attachments = html.split("<div class=\"attachments\">").nth(1).unwrap();
                            assert!(attachments.starts_with("<div class=\"attachment attachment-image\">"));
                            assert!(attachments.find("<img ").unwrap() < attachments.find("<details ").unwrap());
                    }).join().unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "render_html_report" {
            case "Renders image evidence as an inline preview" {
                verify "render an image linked to its evidence file" {
                    std::thread::spawn(|| {
                        let mut result = report(CaseOutcome::Passed);
                        result.evidence.push(Evidence::Image {
                            name: "Profile page".to_owned(),
                            path: "screenshots/profile.png".to_owned(),
                        });

                        let html = render_html_report(
                            vec![result],
                            Path::new("/workspace/target/test-reports/linux-system/ubuntu"),
                            Some(Path::new("/workspace")),
                            None,
                            false,
                        );

                        assert!(!html.contains("<base "));
                        assert!(html.contains("class=\"attachment attachment-image\""));
                        assert!(!html.contains("href=\"screenshots/profile.png\""));
                        assert!(html.contains("src=\"screenshots/profile.png\""));
                        assert!(html.contains("alt=\"Profile page\""));
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    // An absolute path would otherwise be linked relative to the project and
    // resolve to a location that does not exist next to the document.
    keyringd_testkit::scenario! {
        group "render_html_report" {
            case "Links absolute evidence relative to the report directory" {
                verify "express an absolute evidence file as a relative HTML link" {
                    std::thread::spawn(|| {
                        let mut result = report(CaseOutcome::Passed);
                        result.evidence.push(Evidence::Image {
                            name: "Profile page".to_owned(),
                            path: "/workspace/target/test-reports/screenshots/profile.png".to_owned(),
                        });

                        let html = render_html_report(
                            vec![result],
                            Path::new("/workspace/target/test-reports"),
                            Some(Path::new("/workspace")),
                            None,
                            false,
                        );

                        assert!(html.contains("src=\"screenshots/profile.png\""));
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "render_html_report" {
            case "Renders evidence inside its step block" {
                verify "place the evidence alongside the step that recorded it" {
                    std::thread::spawn(|| {
                        let mut result = report(CaseOutcome::Passed);
                        result.steps.push(StepResult {
                            kind: StepKind::When,
                            description: "the page is captured".to_owned(),
                            source: SourceLocation::new("tests/example.rs", 14, 5),
                            outcome: StepOutcome::Passed,
                            duration_ms: 1,
                            evidence: vec![Evidence::Image {
                                name: "Captured page".to_owned(),
                                path: "screenshots/page.png".to_owned(),
                            }],
                        });

                        let html = render_html_report(
                            vec![result],
                            Path::new("target/test-reports"),
                            None,
                            None,
                            false,
                        );

                        let step = html.find("class=\"step-block\"").unwrap();
                        let evidence = html.find("class=\"step-evidence-row\"").unwrap();
                        let image = html.find("alt=\"Captured page\"").unwrap();
                        let block_end = html[evidence..].find("</tbody>").unwrap() + evidence;
                        assert!(step < evidence && evidence < image && image < block_end);
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "render_html_report" {
            case "Nests grouped cases below the source file in the toc" {
                verify "list grouped cases beneath the reporting source file in the contents tree" {
                    std::thread::spawn(|| {
                        let mut grouped = report(CaseOutcome::Passed);
                        grouped.title = "decodes an identities request".to_owned();
                        grouped.group = Some("SshAgentRequest::from_bytes".to_owned());

                        let html = render_html_report(
                            vec![grouped],
                            Path::new("target/test-reports"),
                            None,
                            None,
                            false,
                        );

                        assert!(html.contains("<div class=\"toc-file\" data-targets=\"group-0\">"));
                        assert!(html.contains(">example.rs</button>"));
                        assert!(!html.contains(">example.rs</summary>"));
                        assert!(!html.contains("Ungrouped"));
                        assert!(html.contains("class=\"toc-link toc-file-link\""));
                        assert!(html.contains("class=\"toc-link toc-group\""));
                        assert!(html.contains(">SshAgentRequest::from_bytes</button>"));
                        assert!(html.contains("class=\"toc-link toc-case\""));
                        assert!(html.contains("data-target=\"case-0\""));
                        assert!(html.contains(">decodes an identities request</button>"));
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "render_html_report" {
            case "Displays a project relative case path without step paths" {
                verify "show the relative case path and omit redundant paths from step blocks" {
                    std::thread::spawn(|| {
                        let mut report = report(CaseOutcome::Passed);
                        report.source = SourceLocation::new(r"C:\workspace\project\crate\tests\case.rs", 12, 1);
                        report.steps.push(StepResult {
                            kind: StepKind::Given,
                            description: "state exists".to_owned(),
                            source: SourceLocation::new(r"C:\workspace\project\crate\tests\case.rs", 14, 5),
                            outcome: StepOutcome::Passed,
                            duration_ms: 1,
                            evidence: Vec::new(),
                        });

                        let html = render_html_report(
                            vec![report],
                            Path::new(r"C:\workspace\project\target\test-reports"),
                            Some(Path::new(r"C:\workspace\project")),
                            None,
                            false,
                        );

                        assert!(!html.contains("<base "));
                        assert!(html.contains("crate/tests/case.rs:12"));
                        assert!(html.contains("href=\"../../crate/tests/case.rs#L12\""));
                        assert!(html.contains(">L14</a>"));
                        assert!(html.contains("class=\"step-row\""));
                        assert!(html.contains("role=\"button\""));
                        assert!(html.contains("aria-controls=\"case-0-source\""));
                        assert!(html.contains("aria-expanded=\"false\""));
                        assert!(html.contains("<span>Test source</span>"));
                        assert!(html.contains("<details id=\"case-0-source\" class=\"case-code\""));
                        assert!(html.contains("class=\"case-code-source\""));
                        assert!(html.contains("target=\"_blank\" rel=\"noopener noreferrer\""));
                        assert!(html.contains("aria-label=\"Open test source in a new tab\""));
                        assert!(!html.contains(">crate/tests/case.rs:12</a>"));
                        assert!(html.contains("data-copy-text=\"crate/tests/case.rs:12\""));
                        assert!(html.contains("aria-label=\"Copy source path\""));
                        assert!(!html.contains("class=\"step-code\""));
                        assert!(html.contains("<code class=\"language-rust\">"));
                        assert!(html.contains("let state = State::new();"));
                        assert!(!html.contains("file:///"));
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "render_html_report" {
            case "Links source locations below the configured revision url" {
                verify "build source links from the supplied revision base and line locations" {
                    std::thread::spawn(|| {
                        let mut report = report(CaseOutcome::Passed);
                        report.source = SourceLocation::new(r"C:\workspace\project\crate\tests\case.rs", 12, 1);

                        let html = render_html_report(
                            vec![report],
                            Path::new(r"C:\workspace\project\target\test-reports"),
                            Some(Path::new(r"C:\workspace\project")),
                            Some("https://github.example/project/blob/commit/"),
                            false,
                        );

                        assert!(html.contains(
                            "href=\"https://github.example/project/blob/commit/crate/tests/case.rs#L12\""
                        ));
                        assert!(html.contains("data-copy-text=\"crate/tests/case.rs:12\""));
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "render_html_report" {
            case "Generates an index from json results in source order" {
                verify "order the generated index by source location after loading JSON results" {
                    std::thread::spawn(|| {
                        let directory = tempfile::tempdir().unwrap();
                        let results = directory.path().to_path_buf();
                        fs::create_dir_all(&results).unwrap();
                        let source_file = directory.path().join("cases.rs");
                        fs::write(&source_file, "passing source\nfailing source\n").unwrap();

                        let mut stale = report(CaseOutcome::Passed);
                        stale.title = "stale case from previous run".to_owned();
                        stale.source_code = Some("stale source from previous run".to_owned());
                        stale.source = SourceLocation::new(source_file.to_string_lossy(), 1, 1);
                        fs::write(
                            results.join("stale.json"),
                            serde_json::to_vec(&stale).unwrap(),
                        )
                        .unwrap();

                        let mut passed = report(CaseOutcome::Passed);
                        passed.title = "passing case".to_owned();
                        passed.source_code = Some("passing source".to_owned());
                        passed.source = SourceLocation::new(source_file.to_string_lossy(), 1, 1);
                        let mut failed = report(CaseOutcome::Failed);
                        failed.title = "failing case".to_owned();
                        failed.source_code = Some("failing source".to_owned());
                        failed.source = SourceLocation::new(source_file.to_string_lossy(), 2, 1);
                        failed.failure = Some("expected failure details".to_owned());
                        fs::write(
                            results.join("passed.json"),
                            serde_json::to_vec(&passed).unwrap(),
                        )
                        .unwrap();
                        fs::write(
                            results.join("failed.json"),
                            serde_json::to_vec(&failed).unwrap(),
                        )
                        .unwrap();

                        crate::generate_html_report(directory.path()).unwrap();

                        let html = fs::read_to_string(directory.path().join("index.html")).unwrap();
                        assert!(html.contains("2 cases · 1 passed · 1 failed · 0 skipped"));
                        assert!(html.contains("expected failure details"));
                        assert!(html.find("passing case").unwrap() < html.find("failing case").unwrap());
                        assert!(!html.contains("stale case from previous run"));
                        assert!(!html.contains("stale source from previous run"));
                        assert!(!results.join("stale.json").exists());
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "render_html_report" {
            case "Execution metadata is visible and escaped without inventing missing versions" {
                verify "escape collected metadata and leave unavailable version values empty" {
                    std::thread::spawn(|| {
                        let directory = tempfile::tempdir().unwrap();
                        let run = crate::runner::ReportRunMetadata {
                            run_id: "run-42".into(),
                            started_at_utc: Ok("2026-10-01T01:02:03Z".into()),
                            operating_system: "macos".into(),
                            os_version: Ok("macOS test version".into()),
                            test_environment: None,
                            source_revision: Ok("revision-42".into()),
                            source_dirty: Ok(true),
                            tools: vec![crate::runner::ToolExecutionMetadata {
                                name: "SSH server".into(),
                                path: Some("/usr/sbin/sshd".into()),
                                version: Err("<unavailable>".into()),
                            }],
                        };
                        run.write(directory.path()).unwrap();
                        let html = render_html_report(Vec::new(), directory.path(), None, None, false);
                        assert!(html.contains("2026-10-01T01:02:03Z"));
                        assert!(html.contains("revision-42"));
                        assert!(html.contains("/usr/sbin/sshd"));
                        assert!(html.contains("Unavailable: &lt;unavailable&gt;"));
                        assert!(!html.contains("<unavailable>"));
                        assert!(html.contains("run-metadata.json"));
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
