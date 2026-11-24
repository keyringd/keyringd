use crate::RunnerOptions;
use crate::environment::{AuthenticatorEnvironment, ConsoleGuard, RunLock, Workspace, WslTestbed};
use anyhow::{Context, Result, ensure};
use keyringd_system_tests::environment::windows::TestApp;
use keyringd_testkit::runner::{ScenarioResults, ScenarioSelection};
use keyringd_wsl_system_tests::WslPackageInstallation;
use std::path::PathBuf;
use std::process::Command;

pub(crate) fn run(
    options: &RunnerOptions,
    distribution: &str,
    package_directory: &std::path::Path,
) -> Result<()> {
    ensure!(
        options.workspace_root.is_absolute() && options.workspace_root.join("Cargo.toml").is_file(),
        "workspace root must contain Cargo.toml"
    );
    ensure!(
        options.report_directory.is_absolute(),
        "report directory must be absolute"
    );
    let local =
        PathBuf::from(std::env::var_os("LOCALAPPDATA").context("LOCALAPPDATA is required")?);
    ensure!(
        !local.join("Programs/keyringd/keyringd.exe").exists(),
        "WSL system tests require Windows keyringd to be uninstalled"
    );
    // The Windows and WSL suites share UI, USB/IP and the production Agent pipe.
    let cache = local.join("keyringd-ui-tests");
    std::fs::create_dir_all(&cache)?;
    let _lock = RunLock::acquire(&cache)?;
    TestApp::require_agent_pipe_available()?;
    ensure!(
        package_directory.is_dir(),
        "WSL package directory is missing: {}",
        package_directory.display()
    );
    let testbed = WslTestbed::prepare(&options.workspace_root, distribution)?;
    let result = run_cases(options, testbed.installation(), package_directory);
    let cleanup = testbed.close();
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => {
            Err(error.context(format!("WSL cleanup also failed: {cleanup:#}")))
        }
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn run_cases(
    options: &RunnerOptions,
    installation: &WslPackageInstallation,
    package_directory: &std::path::Path,
) -> Result<()> {
    let installed = installation.install(package_directory)?;
    std::fs::write(
        options.report_directory.join("package-installation.log"),
        installed,
    )?;
    let companion = installation.companion_path()?;
    ensure!(
        companion.is_file(),
        "installed WSL companion is missing at {}",
        companion.display()
    );
    let workspace = Workspace::new(options)?;
    let scenarios = crate::build::build_scenarios(&workspace, "keyringd-wsl-system-tests")?;
    let mut list = Command::new(&scenarios);
    list.current_dir(&options.workspace_root).arg("--list");
    if let Some(filter) = &options.filter {
        list.arg(filter);
    }
    let output = list.output()?;
    ensure!(output.status.success(), "WSL scenario listing failed");
    let selection = ScenarioSelection::from_listing(&String::from_utf8(output.stdout)?)?;
    keyringd_system_tests::authenticator::windows::usbip::WindowsUsbIpClient::installed()?
        .probe()?;
    let authenticator = AuthenticatorEnvironment::prepare(options)?;
    let _console = ConsoleGuard::minimize();
    let mut failed = Vec::new();
    for case in selection.cases() {
        println!("{case}");
        let mut command = Command::new(&scenarios);
        super::case_runner::configure_environment(
            &mut command,
            options,
            &companion,
            None,
            &authenticator,
        );
        command
            .current_dir(&options.workspace_root)
            .args(["--test-threads=1", "--exact", case])
            .env("KEYRINGD_TEST_WSL_DISTRO", installation.distribution());
        let log = super::authenticator_log_capture::AuthenticatorLogCapture::start(
            &options.report_directory,
        )?;
        let passed = super::case_runner::run(&mut command, options, case, &companion)?;
        if !passed {
            log.attach_to_failed_case(
                &options.report_directory,
                &format!("keyringd-wsl-system-tests::scenarios::{case}"),
            )?;
            failed.push(case.clone());
        }
    }
    let results = ScenarioResults::load(
        &options.report_directory,
        "keyringd-wsl-system-tests::scenarios::",
    )?;
    println!(
        "WSL {} system scenarios: {results}",
        installation.distribution()
    );
    results.verify(&selection)?;
    ensure!(
        failed.is_empty(),
        "WSL scenario processes failed: {}",
        failed.join(", ")
    );
    Ok(())
}
