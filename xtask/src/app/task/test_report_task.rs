use crate::core::service;
use std::path::PathBuf;

use clap::Args;

#[derive(Args, Debug)]
pub(crate) struct TestReportTask {
    /// Directory containing recorded results grouped by category and OS.
    pub(crate) results: PathBuf,

    /// Destination directory for the generated report site.
    pub(crate) output: PathBuf,

    /// Repository URL prefix used for links to the tested source revision.
    #[arg(long, value_parser = clap::builder::NonEmptyStringValueParser::new())]
    pub(crate) source_base_url: Option<String>,
}

impl TestReportTask {
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        service::test_report::generate_site(&self.results, &self.output, self.source_base_url)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{TaskCommand, parse};

    keyringd_testkit::scenario! {
        group "parse" {
            case "Accepts report input output and source base url" {
                verify "parse the result directory, output directory, and source revision link" {
                    let cli = parse([
                        "xtask",
                        "test-report",
                        "--source-base-url",
                        "https://example.test/repository/blob/revision",
                        "target/custom-results",
                        "target/custom-reports",
                    ])
                    .unwrap();

                    let TaskCommand::TestReport(task) = cli else {
                        panic!("expected the test-report task");
                    };
                    assert_eq!(
                        task.results,
                        std::path::PathBuf::from("target/custom-results")
                    );
                    assert_eq!(
                        task.output,
                        std::path::PathBuf::from("target/custom-reports")
                    );
                    assert_eq!(
                        task.source_base_url.as_deref(),
                        Some("https://example.test/repository/blob/revision")
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "parse" {
            case "Rejects report option without its value" {
                verify "reject a source link option with no value" {
                    assert!(
                        parse([
                            "xtask",
                            "test-report",
                            "target/custom-results",
                            "target/custom-reports",
                            "--source-base-url",
                        ])
                        .is_err()
                    );
                }
            }
        }
    }
}
