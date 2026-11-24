#[cfg(windows)]
use keyringd_system_tests::application::ApplicationConfiguration;
#[cfg(windows)]
use keyringd_system_tests::application::SystemTestApplication;
#[cfg(windows)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
#[cfg(target_os = "windows")]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
#[cfg(target_os = "windows")]
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
};

#[cfg(windows)]
use crate::scenario_steps;
#[cfg(windows)]
use keyringd_system_tests::application::features::qr_prompt_window::QrPromptWindowDriver;
#[cfg(windows)]
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
#[cfg(windows)]
use keyringd_system_tests::environment::SystemTestEnvironment;
#[cfg(windows)]
use keyringd_system_tests::ssh::SshServers;
#[cfg(windows)]
use keyringd_system_tests::ssh::SshSession;
#[cfg(windows)]
use keyringd_system_tests::ssh::SshTestServer;
#[cfg(target_os = "windows")]
use keyringd_testkit::evidence_log;
use keyringd_testkit::scenario;

#[cfg(target_os = "windows")]
use keyringd_core::service::DeviceKeyProtectionChange as WindowsDeviceKeyProtectionChange;
#[cfg(target_os = "windows")]
use keyringd_system_tests::application::features::device_key_security_window::windows::WindowsDeviceKeySecurityWindow;

scenario! {
    group "SSH Authentication / CNG Device Key" {
        #[cfg(target_os = "windows")]
        case "Prompts for a CNG-protected Device Key password in Windows Security during SSH authentication" {
            skip_if: !NativeSystemTestEnvironment::security_automation_is_elevated().unwrap() => "Need an elevated Windows UIA test environment";
            given "Create a Device Key with the Windows CNG Software KSP" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let _key = environment
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                environment
                    .application()
                    .start(ApplicationConfiguration {
                        software_cng: true,
                        ..Default::default()
                    })
                    .unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                let created = (|| -> anyhow::Result<()> {
                    scenario_steps::profile_setup::form::submit_creation(
                        &mut environment,
                        "cng-password",
                    )?;
                    environment
                        .application()
                        .profile_setup_window()
                        .wizard()?
                        .interaction_page()?
                        .qr_view()?
                        .press_use_security_key()?;
                    environment
                        .application()
                        .profile_setup_window()
                        .wizard()?
                        .completion_page()?
                        .done()?;
                    scenario_steps::profile_management::profile_list::assert_visible(
                        &environment.application().profiles_window()?,
                        "cng-password",
                    )
                })();
                if created.is_err() {
                    environment
                        .application()
                        .capture_screenshot("CNG profile setup failure")
                        .unwrap();
                }
                created.unwrap();
                let ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut environment,
                    "cng-password",
                )
                .unwrap();
            }
            when "Create a Key-only Password in Windows Security" {
                {
                    let previous = environment.application().stored_profiles().unwrap()[0]["deviceKey"]["handle"].clone();
                    let confirmation = environment
                        .application()
                        .profiles_window()
                        .unwrap()
                        .open_device_key_protection(
                            "cng-password",
                            WindowsDeviceKeyProtectionChange::Enable,
                        )
                        .unwrap();
                    confirmation.continue_change().unwrap();
                    let page = WindowsDeviceKeySecurityWindow::wait_open("cng-password").unwrap();
                    page.wait_for_password_creation().unwrap();
                    page.capture_screenshot("Create CNG key password").unwrap();
                    page.create_password("CngTestPass1!").unwrap();
                    page.wait_closed().unwrap();
                    environment
                        .application()
                        .qr_prompt_window()
                        .unwrap()
                        .press_use_security_key()
                        .unwrap();
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
                    loop {
                        let stored = environment.application().stored_profiles().unwrap();
                        if stored[0]["deviceKey"]["localAuthentication"] == "required"
                            && stored[0]["deviceKey"]["handle"] != previous
                        {
                            break;
                        }
                        assert!(
                            std::time::Instant::now() < deadline,
                            "CNG protected Device Key was not persisted: {stored:?}"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(25));
                    }
                    environment
                        .application()
                        .trust_entries_window("cng-password")
                        .unwrap()
                        .close()
                        .unwrap();
                }
            }
            then "Another Device Key with a CNG protection policy is saved" {
                let details = environment
                    .application()
                    .profiles_window()
                    .unwrap()
                    .open_details("cng-password")
                    .unwrap();
                assert_eq!(
                    details.details().unwrap().protection,
                    "Local authentication required"
                );
                details.close().unwrap();
                assert_eq!(
                    scenario_steps::profile_management::public_keys::copy_ca(
                        &mut environment,
                        "cng-password"
                    )
                    .unwrap(),
                    ca
                );
            }
            when "Restart the app with the same save destination" {
                environment.application().restart().unwrap();
                let trust = scenario_steps::profile_management::public_keys::copy_device_key(
                    &mut environment,
                    "cng-password",
                )
                .unwrap();
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| identity.algorithm == "ecdsa-sha2-nistp256")
                    .unwrap();
                let server = environment.ssh_servers().start(&trust).unwrap();
                let client = server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        &identity,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap();
                let page = WindowsDeviceKeySecurityWindow::wait_open("cng-password").unwrap();
                page.capture_screenshot("CNG password required for SSH")
                    .unwrap();
                page.enter_password("CngTestPass1!").unwrap();
            }
            then "Password Input screen closes" {
                page.wait_closed().unwrap();
            }
            when "Waiting for SSH Client to complete authentication" {
                let output = client.finish().unwrap();
            }
            then "Windows Security Requests Password and SSH Authentication Successful" {
                output.assert_authenticated(&identity);
                evidence_log!("CNG-protected SSH", &output.server_log);
            }
        }
    }

    group "SSH Authentication / CNG Device Key" {
        #[cfg(target_os = "windows")]
        case "Rejects the old CNG Device Key password and authenticates with the new one after reset" {
            skip_if: !NativeSystemTestEnvironment::security_automation_is_elevated().unwrap() => "Need an elevated Windows UIA test environment";
            given "Profile with old Password protected CNG device key" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let _key = environment
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                environment
                    .application()
                    .start(ApplicationConfiguration {
                        software_cng: true,
                        ..Default::default()
                    })
                    .unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                scenario_steps::profile_setup::form::submit_creation(
                    &mut environment,
                    "cng-password-change",
                )
                .unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .interaction_page()
                    .unwrap()
                    .qr_view()
                    .unwrap()
                    .press_use_security_key()
                    .unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "cng-password-change",
                )
                .unwrap();
                let ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut environment,
                    "cng-password-change",
                )
                .unwrap();
                {
                    let previous = environment.application().stored_profiles().unwrap()[0]["deviceKey"]["handle"].clone();
                    let confirmation = environment
                        .application()
                        .profiles_window()
                        .unwrap()
                        .open_device_key_protection(
                            "cng-password-change",
                            WindowsDeviceKeyProtectionChange::Enable,
                        )
                        .unwrap();
                    confirmation.continue_change().unwrap();
                    let page =
                        WindowsDeviceKeySecurityWindow::wait_open("cng-password-change").unwrap();
                    page.wait_for_password_creation().unwrap();
                    page.capture_screenshot("Initial CNG password").unwrap();
                    page.create_password("CngOldPass2!").unwrap();
                    page.wait_closed().unwrap();
                    environment
                        .application()
                        .qr_prompt_window()
                        .unwrap()
                        .press_use_security_key()
                        .unwrap();
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
                    loop {
                        let stored = environment.application().stored_profiles().unwrap();
                        if stored[0]["deviceKey"]["localAuthentication"] == "required"
                            && stored[0]["deviceKey"]["handle"] != previous
                        {
                            break;
                        }
                        assert!(
                            std::time::Instant::now() < deadline,
                            "CNG protected Device Key was not persisted: {stored:?}"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(25));
                    }
                    environment
                        .application()
                        .trust_entries_window("cng-password-change")
                        .unwrap()
                        .close()
                        .unwrap();
                }
                let old_key = {
                    scenario_steps::profile_management::public_keys::copy_device_key(
                        &mut environment,
                        "cng-password-change",
                    )
                    .unwrap()
                };
            }
            when "Create a new CNG Device Key for Password changes" {
                {
                    let previous = environment.application().stored_profiles().unwrap()[0]["deviceKey"]["handle"].clone();
                    let confirmation = environment
                        .application()
                        .profiles_window()
                        .unwrap()
                        .open_device_key_protection(
                            "cng-password-change",
                            WindowsDeviceKeyProtectionChange::ChangePassword,
                        )
                        .unwrap();
                    confirmation.continue_change().unwrap();
                    let page =
                        WindowsDeviceKeySecurityWindow::wait_open("cng-password-change").unwrap();
                    page.wait_for_password_creation().unwrap();
                    page.capture_screenshot("Changed CNG password").unwrap();
                    page.create_password("CngNewPass3!").unwrap();
                    page.wait_closed().unwrap();
                    environment
                        .application()
                        .qr_prompt_window()
                        .unwrap()
                        .press_use_security_key()
                        .unwrap();
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
                    loop {
                        let stored = environment.application().stored_profiles().unwrap();
                        if stored[0]["deviceKey"]["localAuthentication"] == "required"
                            && stored[0]["deviceKey"]["handle"] != previous
                        {
                            break;
                        }
                        assert!(
                            std::time::Instant::now() < deadline,
                            "CNG protected Device Key was not persisted: {stored:?}"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(25));
                    }
                    environment
                        .application()
                        .trust_entries_window("cng-password-change")
                        .unwrap()
                        .close()
                        .unwrap();
                }
                environment.application().restart().unwrap();
            }
            then "The original key is replaced and the CA is maintained" {
                let current_key = scenario_steps::profile_management::public_keys::copy_device_key(
                    &mut environment,
                    "cng-password-change",
                )
                .unwrap();
                assert_ne!(old_key, current_key);
                assert_eq!(
                    scenario_steps::profile_management::public_keys::copy_ca(
                        &mut environment,
                        "cng-password-change"
                    )
                    .unwrap(),
                    ca
                );
            }
            when "Try an SSH signature with the old Password" {
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| identity.algorithm == "ecdsa-sha2-nistp256")
                    .unwrap();
                let server = environment.ssh_servers().start(&current_key).unwrap();
                let client = server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        &identity,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap();
                let mut page =
                    WindowsDeviceKeySecurityWindow::wait_open("cng-password-change").unwrap();
                page.enter_password("CngOldPass2!").unwrap();
            }
            then "Signed with Old Password is rejected" {
                page.wait_for_password_rejection().unwrap();
            }
            when "Approve SSH Signatures with New Password" {
                page.capture_screenshot("Old CNG password rejected")
                    .unwrap();
                page.enter_password("CngNewPass3!").unwrap();
            }
            then "Password Input screen closes" {
                page.wait_closed().unwrap();
            }
            when "Waiting for SSH Client to complete authentication" {
                let output = client.finish().unwrap();
            }
            then "Old Password is denied and can be SSH authenticated with the new Password" {
                output.assert_authenticated(&identity);
                evidence_log!("CNG password changed SSH", &output.server_log);
            }
        }
    }
}
