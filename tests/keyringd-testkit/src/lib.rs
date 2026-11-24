pub mod accessibility_traversal;
mod application;
mod execution;
mod infrastructure;
pub mod macos_ui_protocol;
mod model;
pub mod product_version;
mod qr_code_reader;
pub use qr_code_reader::QrCodeReader;
pub mod agent;
mod report;
pub mod requirements_report;
pub mod runner;
pub mod ssh;
pub mod supervision;

pub use application::ReportGenerationError;
pub use execution::record_image;
pub use infrastructure::ReportInfrastructureError;
pub use keyringd_testkit_macros::{
    evidence_bytes, evidence_file, evidence_image, evidence_log, evidence_text, scenario,
};
pub use report::generate_report_site;

use std::path::Path;

mod report_options;
pub use report_options::ReportOptions;

pub fn default_report_directory() -> std::path::PathBuf {
    infrastructure::ResultStore::default_report_directory()
}

pub fn generate_html_report(directory: impl AsRef<Path>) -> Result<(), ReportGenerationError> {
    generate_html_report_with_options(directory, &ReportOptions::default())
}

pub fn generate_html_report_with_options(
    directory: impl AsRef<Path>,
    options: &ReportOptions,
) -> Result<(), ReportGenerationError> {
    let directory = directory.as_ref();
    let results = infrastructure::ResultStore::for_report_directory(directory).load_current()?;
    let project_root = infrastructure::ProjectRootLocator::locate(&results);
    let html = report::render_html_report(
        results,
        directory,
        project_root.as_deref(),
        options.source_base_url.as_deref(),
        options.include_requirements,
    );
    std::fs::create_dir_all(directory).map_err(|source| {
        ReportGenerationError::CreateDirectory {
            path: directory.to_path_buf(),
            source,
        }
    })?;
    let output = directory.join("index.html");
    std::fs::write(&output, html).map_err(|source| ReportGenerationError::HtmlWrite {
        path: output,
        source,
    })?;
    if options.include_requirements
        && let Some(project_root) = project_root.filter(|root| directory.starts_with(root))
    {
        let requirements = project_root.join("docs/internal/requirements.yaml");
        if requirements.is_file() {
            requirements_report::generate(&requirements_report::ReportPaths {
                requirements,
                test_results: directory.to_path_buf(),
                output: directory.join("requirements.html"),
            })?;
        }
    }
    Ok(())
}

#[doc(hidden)]
pub mod __private {
    pub use crate::application::{
        CaseMetadata, begin_case, capture, capture_async, capture_async_with_timeout, complete_case,
    };
    pub use crate::execution::CaseExecution;
    pub use crate::execution::EvidenceScope;
    pub use crate::model::{SourceLocation, StepKind};
}

mod pxp_advertisement_relay;
pub use pxp_advertisement_relay::PxpAdvertisementRelay;

#[cfg(test)]
extern crate self as keyringd_testkit;
