use crate::core::driver::ReportSession;
use crate::core::driver::cargo;
use crate::core::driver::test_runner;
use crate::core::model::{HostPlatform, SystemTestRequest, TestRunOutcome, Workspace};
use anyhow::Result;
use std::{ffi::OsString, path::Path};

pub(crate) fn standard_with_report(workspace: &Workspace, arguments: &[OsString]) -> Result<()> {
    let report = ReportSession::start(
        workspace,
        crate::core::driver::test_report::directory_from_env()
            .unwrap_or_else(|| workspace.default_report_directory()),
    )?;
    with_report(&report, arguments.is_empty(), |directory| {
        vec![("cargo test", cargo::test(workspace, arguments, directory))]
    })
}

pub(crate) fn host_tests_with_report(
    workspace: &Workspace,
    linux: SystemTestRequest,
    arguments: &[OsString],
) -> Result<()> {
    let request = SystemTestRequest::for_host(HostPlatform::parse(std::env::consts::OS)?, linux);
    let report = ReportSession::start(workspace, workspace.default_report_directory())?;
    with_report(&report, arguments.is_empty(), |directory| {
        vec![
            ("cargo test", cargo::test(workspace, arguments, directory)),
            (
                request.name(),
                test_runner::run_suite(workspace, &request, directory),
            ),
        ]
    })
}

fn with_report(
    report: &ReportSession<'_>,
    include_requirements: bool,
    tests: impl FnOnce(&Path) -> Vec<(&'static str, Result<()>)>,
) -> Result<()> {
    let stages = tests(report.directory());
    // Always attempt rendering, even when one or more test stages failed.
    TestRunOutcome::new(stages, report.render(include_requirements)).into_result()
}
