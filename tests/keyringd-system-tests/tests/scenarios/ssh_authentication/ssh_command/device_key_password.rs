#[cfg(target_os = "linux")]
use crate::scenario_steps;
#[cfg(target_os = "linux")]
use keyringd_system_tests::application::ApplicationConfiguration;
#[cfg(target_os = "linux")]
use keyringd_system_tests::application::SystemTestApplication;
#[cfg(target_os = "linux")]
use keyringd_system_tests::application::features::profile_setup::CompletionPageDriver;
#[cfg(target_os = "linux")]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
#[cfg(target_os = "linux")]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
#[cfg(target_os = "linux")]
use keyringd_system_tests::authenticator::ApprovalMode;
#[cfg(target_os = "linux")]
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
#[cfg(target_os = "linux")]
use keyringd_system_tests::environment::SystemTestEnvironment;

#[cfg(target_os = "linux")]
use keyringd_system_tests::ssh::SshServers;
#[cfg(target_os = "linux")]
use keyringd_system_tests::ssh::SshSession;
#[cfg(target_os = "linux")]
use keyringd_system_tests::ssh::SshTestServer;
#[cfg(target_os = "linux")]
use keyringd_test_usbip::authenticator_server::client::AuthenticatorProfile;
#[cfg(target_os = "linux")]
use keyringd_testkit::evidence_log;
use keyringd_testkit::scenario;
#[cfg(target_os = "linux")]
const PASSWORD: &str = "correct horse battery staple";
#[cfg(target_os = "linux")]
const CHANGED_PASSWORD: &str = "troubadour battery staple correct";
#[cfg(target_os = "linux")]
const PROFILE_NAME: &str = "password-protected-device-key";
scenario! {
    group "SSH Authentication / Device Key Password" {
        #[cfg(target_os = "linux")]
        case "Prompts for the protected Device Key password during authentication" {
            given "Password Prepare a Secured Device Key and a Temporary SSHD to Trust Its Public Key" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let key = environment
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                key.authenticator().reset(AuthenticatorProfile::UvConfiguredPinUnconfigured)
                    .unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                environment
                    .application()
                    .start(ApplicationConfiguration::default())
                    .unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                scenario_steps::profile_setup::security_key_profiles::create_with_built_in_uv(
                    &mut environment,
                    &key,
                    PROFILE_NAME,
                );
                environment
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
                let device_key = scenario_steps::profile_management::public_keys::copy_device_key(
                    &mut environment,
                    PROFILE_NAME,
                )
                .unwrap();
                scenario_steps::profile_management::linux_device_key_password::enable(
                    &mut environment,
                    PROFILE_NAME,
                    PASSWORD,
                )
                .unwrap();
                environment.application().restart().unwrap();
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|i| {
                        i.comment.contains(PROFILE_NAME) && i.algorithm == "ecdsa-sha2-nistp256"
                    })
                    .unwrap();
                let server = environment.ssh_servers().start(&device_key).unwrap();
            }
            when "PASSWORD Initiate SSH Authentication with Protected Device Key" {
                let mut ssh = server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        &identity,
                        "printf password-enabled",
                    )
                    .unwrap();
            }
            then "Requires Device Key Password before SSH is complete" {
                environment
                    .application()
                    .device_key_security_window()
                    .unwrap()
                    .wait_open()
                    .unwrap();
                assert!(
                    !ssh.is_finished().unwrap(),
                    "SSH completed before requesting the Device Key password"
                );
                environment
                    .application()
                    .device_key_security_window()
                    .unwrap()
                    .capture_screenshot("Device Key password prompt")
                    .unwrap();
            }
            when "Unlock Device Key with correct Password" {
                environment
                    .application()
                    .device_key_security_window()
                    .unwrap()
                    .unlock(PASSWORD)
                    .unwrap();
                let connection = ssh.finish().unwrap();
            }
            then "Password Successful SSH authentication with protected Device Key" {
                assert!(connection.status.success(), "{}", connection.stderr);
                assert_eq!(connection.stdout, "password-enabled");
                evidence_log!("Password-protected Device Key SSH", &connection.stderr);
                evidence_log!("sshd log", &connection.server_log);
            }
        }
        #[cfg(target_os = "linux")]
        case "Rejects the old Device Key password and authenticates with the new one after a change" {
            given "Password Prepare a Secured Device Key and a Temporary SSHD to Trust Its Public Key" {
                let name = "changed-device-key-password";
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let key = environment
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                key.authenticator().reset(AuthenticatorProfile::UvConfiguredPinUnconfigured)
                    .unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                environment
                    .application()
                    .start(ApplicationConfiguration::default())
                    .unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                scenario_steps::profile_setup::security_key_profiles::create_with_built_in_uv(
                    &mut environment,
                    &key,
                    name,
                );
                environment
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
                let ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut environment,
                    name,
                )
                .unwrap();
                scenario_steps::profile_management::linux_device_key_password::enable(
                    &mut environment,
                    name,
                    PASSWORD,
                )
                .unwrap();
                environment.application().restart().unwrap();
                scenario_steps::profile_management::linux_device_key_password::change(
                    &mut environment,
                    name,
                    PASSWORD,
                    CHANGED_PASSWORD,
                )
                .unwrap();
                environment.application().restart().unwrap();
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|i| i.comment == format!("keyringd {name} device certificate"))
                    .unwrap();
            }
            when "Try the Device Key signature with the old Password" {
                let server = environment.ssh_servers().start(&ca).unwrap();
                let mut ssh = server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        &identity,
                        "printf old-password",
                    )
                    .unwrap();
                environment
                    .application()
                    .device_key_security_window()
                    .unwrap()
                    .wait_open()
                    .unwrap();
                assert!(
                    !ssh.is_finished().unwrap(),
                    "SSH completed before requesting the old Device Key password"
                );
                environment
                    .application()
                    .device_key_security_window()
                    .unwrap()
                    .unlock(PASSWORD)
                    .unwrap();
                let old_password_result = ssh.finish().unwrap();
            }
            then "SSH cannot be authenticated with the old Password" {
                assert!(!old_password_result.status.success());
                evidence_log!(
                    "Rejected old Device Key password",
                    &old_password_result.stderr
                );
            }
            when "Try Device Key Signature with New Password" {
                let server = environment.ssh_servers().start(&ca).unwrap();
                let mut ssh = server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        &identity,
                        "printf new-password",
                    )
                    .unwrap();
                environment
                    .application()
                    .device_key_security_window()
                    .unwrap()
                    .wait_open()
                    .unwrap();
                assert!(
                    !ssh.is_finished().unwrap(),
                    "SSH completed before requesting the new Device Key password"
                );
                environment
                    .application()
                    .device_key_security_window()
                    .unwrap()
                    .capture_screenshot("Changed Device Key password prompt")
                    .unwrap();
                environment
                    .application()
                    .device_key_security_window()
                    .unwrap()
                    .unlock(CHANGED_PASSWORD)
                    .unwrap();
                let new_password_result = ssh.finish().unwrap();
            }
            then "Successful SSH authentication with the new Password" {
                assert!(
                    new_password_result.status.success(),
                    "{}",
                    new_password_result.stderr
                );
                assert_eq!(new_password_result.stdout, "new-password");
                evidence_log!(
                    "Accepted changed Device Key password",
                    &new_password_result.stderr
                );
                evidence_log!("sshd log", &new_password_result.server_log);
            }
        }
    }
}
