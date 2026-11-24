use crate::core::driver::test_report;
use anyhow::Result;
use std::path::Path;

pub(crate) fn generate_site(
    results: &Path,
    output: &Path,
    source_base_url: Option<String>,
) -> Result<()> {
    test_report::generate_site(results, output, source_base_url)?;
    println!("generated {}", output.join("index.html").display());
    Ok(())
}
