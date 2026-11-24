use anyhow::Result;
use std::path::Path;

pub(crate) fn generate_site(
    results: &Path,
    output: &Path,
    source_base_url: Option<String>,
) -> Result<()> {
    keyringd_testkit::generate_report_site(
        results,
        output,
        &keyringd_testkit::ReportOptions {
            source_base_url,
            ..Default::default()
        },
    )?;
    Ok(())
}

pub(in crate::core) fn directory_from_env() -> Option<std::path::PathBuf> {
    std::env::var_os("KEYRINGD_TEST_REPORT_DIR").map(std::path::PathBuf::from)
}
