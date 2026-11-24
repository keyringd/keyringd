#[cfg(windows)]
use super::case_runner;

#[cfg(windows)]
use super::authenticator_log_capture::AuthenticatorLogCapture;
#[cfg(windows)]
use super::case_runner::configure_environment;
#[cfg(windows)]
use crate::RunnerOptions;
#[cfg(windows)]
use crate::{
    environment::AuthenticatorEnvironment,
    environment::RunLock,
    environment::Workspace,
    environment::{ConsoleGuard, OpenSshInstallation, SecurityAutomationManager},
    util::command::checked_output,
};
#[cfg(windows)]
use anyhow::Result;
#[cfg(windows)]
use keyringd_testkit::runner::ScenarioResults;
#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
pub(crate) struct WindowsExecution<'a> {
    options: &'a RunnerOptions,
}
#[cfg(windows)]
impl<'a> WindowsExecution<'a> {
    pub(crate) fn new(options: &'a RunnerOptions) -> Self {
        Self { options }
    }
    pub(crate) fn run(self) -> Result<()> {
        let options = self.options;
        options.validate()?;
        let workspace = Workspace::new(options)?;
        let _lock = RunLock::acquire(&workspace.cache)?;
        keyringd_system_tests::environment::windows::TestApp::require_agent_pipe_available()?;
        let openssh = OpenSshInstallation::installed()?;
        let git_ssh = std::path::PathBuf::from(
            std::env::var_os("ProgramFiles")
                .ok_or_else(|| anyhow::anyhow!("ProgramFiles is not set"))?,
        )
        .join("Git/usr/bin/ssh.exe");
        anyhow::ensure!(
            git_ssh.is_file(),
            "Install Git for Windows to run the Windows system tests"
        );
        keyringd_testkit::runner::ReportRunMetadata::record_tool(
            &options.report_directory,
            keyringd_testkit::runner::ToolExecutionMetadata::capture(
                "Git for Windows SSH client",
                &git_ssh,
                &["-V"],
                &workspace.root,
            ),
        )?;
        for (name, executable) in [("SSH client", "ssh.exe"), ("SSH server", "sshd.exe")] {
            keyringd_testkit::runner::ReportRunMetadata::record_tool(
                &options.report_directory,
                keyringd_testkit::runner::ToolExecutionMetadata::capture(
                    name,
                    &openssh.directory.join(executable),
                    &["-V"],
                    &workspace.root,
                ),
            )?;
        }
        let fixtures = crate::build::build_fixture(&workspace)?;
        keyringd_system_tests::authenticator::windows::usbip::WindowsUsbIpClient::installed()?
            .probe()?;
        let authenticator = AuthenticatorEnvironment::prepare(options)?;
        let _console = ConsoleGuard::minimize();
        let scenarios = crate::build::build_scenarios(&workspace, "keyringd-system-tests")?;
        let mut list = Command::new(&scenarios);
        list.current_dir(&workspace.root).arg("--list");
        if let Some(filter) = &options.filter {
            list.arg(filter);
        }
        let listed = checked_output(&mut list, "Windows UI case listing")?;
        let listed = String::from_utf8_lossy(&listed.stdout);
        anyhow::ensure!(
            listed.lines().any(|line| line.ends_with(": test")),
            "no system test cases matched the filter"
        );
        let selection = keyringd_testkit::runner::ScenarioSelection::from_listing(&listed)?;
        let security = if selection
            .cases()
            .iter()
            .any(|case| requires_security_automation(case))
        {
            Some(SecurityAutomationManager::start(
                &fixtures.security_automation,
                &options.report_directory,
            )?)
        } else {
            None
        };
        let mut failed = Vec::new();
        for case in selection.cases() {
            println!("{case}");
            let mut command = Command::new(&scenarios);
            command
                .current_dir(&workspace.root)
                .args(["--test-threads=1", "--exact", case]);
            configure_environment(
                &mut command,
                options,
                &fixtures.app,
                Some(&openssh.directory),
                &authenticator,
            );
            if let Some(security) = &security {
                security.configure(&mut command);
            }
            let log = AuthenticatorLogCapture::start(&options.report_directory)?;
            if !case_runner::run(&mut command, options, case, &fixtures.app)? {
                log.attach_to_failed_case(
                    &options.report_directory,
                    &format!("keyringd-system-tests::scenarios::{case}"),
                )?;
                failed.push(case.clone());
            }
        }
        let results = ScenarioResults::load(
            &options.report_directory,
            "keyringd-system-tests::scenarios::",
        )?;
        println!("Windows UI scenarios: {results}");
        results.verify(&selection)?;
        anyhow::ensure!(
            failed.is_empty(),
            "Windows UI scenario processes failed: {}",
            failed.join(", ")
        );
        Ok(())
    }
}

#[cfg(any(windows, test))]
fn requires_security_automation(case: &str) -> bool {
    case.starts_with(
        "scenarios::ssh_authentication::ssh_command::device_key_local_authentication::",
    ) || case
        .starts_with("scenarios::profile_setup::create_new_passkey::security_key::configured_pin::")
        || case.starts_with(
            "scenarios::profile_setup::recover_existing_passkey::security_key::configured_pin::",
        )
        || case.starts_with(
            "scenarios::profile_setup::create_new_passkey::security_key::cancellation::",
        )
        || case.starts_with(
            "scenarios::profile_setup::recover_existing_passkey::security_key::cancellation::",
        )
        || case.starts_with("scenarios::profile_setup::create_new_passkey::security_key::success::")
        || case.starts_with(
            "scenarios::profile_setup::create_new_passkey::security_key::multiple_keys::",
        )
        || case.starts_with(
            "scenarios::profile_setup::recover_existing_passkey::security_key::success::",
        )
        || case.starts_with("scenarios::ssh_authentication::ssh_command::passkey::")
        || case.starts_with("scenarios::ssh_authentication::connection_test::passkey::")
}

#[cfg(test)]
mod tests {
    use super::requires_security_automation;
    use keyringd_testkit::runner::ScenarioSelection;

    keyringd_testkit::scenario! {
        group "requires_security_automation" {
            case "Listed security cases request automation" {
                verify "detect the selected case names that need native security-dialog automation" {
                    let selection = ScenarioSelection::from_listing(
                                "scenarios::profile_setup::create_new_passkey::security_key::configured_pin::creates_a_profile_with_a_pin_protected_usb_key_and_authenticates_over_ssh: test\n\
             scenarios::profile_setup::recover_existing_passkey::security_key::cancellation::leaves_no_profile_when_security_key_passkey_discovery_is_canceled: test\n\
             scenarios::ssh_authentication::ssh_command::passkey::authenticates_with_a_registered_passkey: test\n",
                            )
                            .unwrap();
                    assert_eq!(selection.cases().len(), 3);
                    assert!(
                        selection
                            .cases()
                            .iter()
                            .all(|case| requires_security_automation(case))
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "requires_security_automation" {
            case "Hybrid only selection does not request automation" {
                verify "skip native security-dialog automation for a Hybrid-only selection" {
                    assert!(!requires_security_automation(
                        "scenarios::profile_setup::create_new_passkey::hybrid::success::creates_a_passkey_through_the_pxp_ble_advertisement_relay"
                    ));
                }
            }
        }
    }
}
