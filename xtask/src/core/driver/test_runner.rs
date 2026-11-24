use crate::core::model::SystemTestRequest;
use crate::core::model::Workspace;
use anyhow::Result;
use std::path::Path;

pub(crate) fn run_suite(
    workspace: &Workspace,
    request: &SystemTestRequest,
    report: &Path,
) -> Result<()> {
    match request {
        SystemTestRequest::Linux {
            distribution,
            package,
            filter,
        } => {
            #[cfg(target_os = "linux")]
            {
                use keyringd_system_tests_linux_runner::{Runner, RunnerOptions};
                Runner::execute(
                    &RunnerOptions {
                        workspace_root: workspace.root().to_owned(),
                        report_directory: report.to_owned(),
                        distribution: distribution.parse().map_err(anyhow::Error::msg)?,
                        package: package.clone(),
                        filter: filter.clone(),
                    },
                    |request| spawn_testbed(workspace, request),
                )
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = (distribution, package, filter);
                anyhow::bail!("Linux system tests can only run on Linux")
            }
        }
        SystemTestRequest::Macos {
            filter,
            case_timeout_seconds,
        } => keyringd_system_tests_macos_runner::Runner::execute(
            &keyringd_system_tests_macos_runner::RunnerOptions {
                workspace_root: workspace.root().to_owned(),
                report_directory: report.to_owned(),
                filter: filter.clone(),
                case_timeout_seconds: *case_timeout_seconds,
            },
        ),
        SystemTestRequest::Windows { filter } => {
            use keyringd_system_tests_windows_runner::{Runner, RunnerOptions, TestSuite};
            Runner::execute(&RunnerOptions {
                workspace_root: workspace.root().to_owned(),
                report_directory: report.to_owned(),
                filter: filter.clone(),
                suite: TestSuite::Windows,
            })
        }
        SystemTestRequest::Wsl {
            distribution,
            package_directory,
            filter,
        } => {
            use keyringd_system_tests_windows_runner::{Runner, RunnerOptions, TestSuite};
            Runner::execute(&RunnerOptions {
                workspace_root: workspace.root().to_owned(),
                report_directory: report.to_owned(),
                filter: filter.clone(),
                suite: TestSuite::Wsl {
                    distribution: distribution.clone(),
                    package_directory: package_directory
                        .clone()
                        .unwrap_or_else(|| workspace.target().join("wsl-packages")),
                },
            })
        }
    }
}
#[cfg(target_os = "linux")]
fn spawn_testbed(
    workspace: &Workspace,
    request: keyringd_system_tests_linux_runner::TestbedExecutionRequest,
) -> Result<()> {
    use anyhow::{Context, ensure};

    let executable = std::env::current_exe().context("failed to locate the xtask executable")?;
    let mut command = std::process::Command::new("sudo");
    if std::env::var_os("KEYRINGD_COVERAGE_ACTIVE").is_some() {
        command.arg("--preserve-env=LLVM_PROFILE_FILE,CARGO_TARGET_DIR");
    }
    if std::env::var_os("KEYRINGD_E2E_TRACE").is_some() {
        command.arg("--preserve-env=KEYRINGD_E2E_TRACE");
    }
    command
        .arg(executable)
        .arg("run-linux-system-testbed")
        .arg("--distribution")
        .arg(request.distribution.to_string())
        .arg("--workspace")
        .arg(request.workspace)
        .arg("--test-executable")
        .arg(request.test_executable)
        .arg("--report-directory")
        .arg(request.report_directory)
        .arg("--image")
        .arg(request.image)
        .arg("--machine")
        .arg(request.machine)
        .arg("--owner-uid")
        .arg(request.owner_uid.to_string())
        .arg("--owner-gid")
        .arg(request.owner_gid.to_string())
        .arg("--authenticator-control-url")
        .arg(request.authenticator_control_url)
        .current_dir(workspace.root());
    if let Some(package) = request.package {
        command.arg("--package").arg(package);
    }
    if let Some(filter) = request.filter {
        command.arg("--filter").arg(filter);
    }
    let status = command
        .status()
        .context("failed to start the Linux system testbed")?;
    ensure!(status.success(), "Linux system tests failed with {status}");
    Ok(())
}
