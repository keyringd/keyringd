use crate::core::{
    driver::{ReportSession, test_report, test_runner},
    model::{SystemTestRequest, TestRunOutcome, Workspace},
};
use anyhow::Result;

pub(crate) fn run(
    workspace: &Workspace,
    request: &SystemTestRequest,
    render_report: bool,
) -> Result<()> {
    let report = ReportSession::start(workspace, report_directory(workspace, request))?;
    let result = test_runner::run_suite(workspace, request, report.directory());
    let rendered = if render_report {
        report.render(false)
    } else {
        Ok(())
    };
    TestRunOutcome::new(vec![(request.name(), result)], rendered).into_result()
}

pub(crate) fn report_directory(
    workspace: &Workspace,
    request: &SystemTestRequest,
) -> std::path::PathBuf {
    request.report_directory(
        &workspace.default_report_directory(),
        test_report::directory_from_env(),
    )
}
