use keyringd_system_tests::application::ApplicationConfiguration;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupStep;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
};

use crate::scenario_steps;
use keyringd_system_tests::authenticator::ApprovalMode;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_system_tests::ssh::{SshServers, SshSession, SshTestServer};
use keyringd_testkit::{evidence_log, evidence_text, scenario};

scenario! {
    group "Profile Setup / Security Key / Passkey Creation" {
        case "Completes a profile by creating a Passkey on a Security Key" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and the OS Security Key environment";
            given "Prepare manually approved Security Key with built-in UV" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let key = environment
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                let configuration = ApplicationConfiguration::default();
                environment.application().start(configuration).unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
            }
            when "Start creating a new Passkey" {
                scenario_steps::profile_setup::form::submit_creation(
                    &mut environment,
                    "sk-new-profile",
                )
                .unwrap();
                scenario_steps::profile_setup::transport_choices::assert_creation(&mut environment)
                    .unwrap();
                environment
                    .application()
                    .capture_screenshot("Passkey creation choices")
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
                let request = scenario_steps::profile_setup::security_key::wait_for_request(
                    &mut environment,
                    &key,
                )
                .unwrap();
            }
            then "Wait for operation approval on Passkey creation request" {
                assert!(request.user_presence_required);
                evidence_text!("Passkey creation request", format!("{request:?}"));
                scenario_steps::profile_setup::security_key::capture_prompt(
                    &mut environment,
                    "Passkey creation approval prompt",
                )
                .unwrap();
            }
            when "Approve Passkey creation" {
                let requests =
                    scenario_steps::profile_setup::security_key::approve_step_with_presence(
                        &mut environment,
                        &key,
                        ProfileSetupStep::AuthorizeNewDevice,
                    );
                evidence_log!("Passkey creation requests", format!("{requests:?}"));
            }
            then "Waiting for Device Key authorization operation approval with the same Security Key" {
                scenario_steps::profile_setup::progress::wait_for_step(
                    &mut environment,
                    ProfileSetupStep::AuthorizeNewDevice,
                )
                .unwrap();
                let request = scenario_steps::profile_setup::security_key::wait_for_request(
                    &mut environment,
                    &key,
                )
                .unwrap();
                assert!(request.user_presence_required);
                scenario_steps::profile_setup::security_key::capture_prompt(
                    &mut environment,
                    "Device Key authorization prompt",
                )
                .unwrap();
            }
            when "Approve Device Key authorization" {
                let requests =
                    scenario_steps::profile_setup::security_key::approve_step_with_presence(
                        &mut environment,
                        &key,
                        ProfileSetupStep::Completed,
                    );
                evidence_log!("Device Key authorization requests", format!("{requests:?}"));
                key.authenticator().set_approval_mode(ApprovalMode::Automatic).unwrap();
                scenario_steps::profile_setup::completion::wait_for(&mut environment).unwrap();
                environment
                    .application()
                    .capture_screenshot("Profile created")
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
            }
            then "Only one item is saved and a certificate is created from the actual key" {
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "sk-new-profile",
                )
                .unwrap();
                let stored = environment.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "sk-new-profile");
                assert!(key.authenticator().request_count().unwrap() > 0);
                let ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut environment,
                    "sk-new-profile",
                )
                .unwrap();
                assert!(
                    ca
                        .starts_with("cert-authority,principals=\"keyringd:"),
                    "{}",
                    ca
                );
                assert!(
                    ca.contains(" sk-ecdsa-sha2-nistp256@openssh.com "),
                    "{}",
                    ca
                );
                assert!(ca.ends_with(" keyringd-ca"), "{}", ca);
            }
            when "Connect a real SSH with the created Certificate" {
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| {
                        identity.comment == "keyringd sk-new-profile device certificate"
                    })
                    .unwrap();
                let connection_server = environment.ssh_servers().start(&ca).unwrap();
                let connection = connection_server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        &identity,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap()
                    .finish()
                    .unwrap();
            }
            then "Published Certificates accepted and can execute commands" {
                connection.assert_authenticated(&identity);
                scenario_steps::profile_setup::security_key::assert_operation_finished(
                    &mut environment,
                    &key,
                )
                .unwrap();
                evidence_log!("Created profile SSH", &connection.stderr);
                evidence_log!("sshd authentication log", &connection.server_log);
                environment
                    .application()
                    .capture_screenshot("Created SSH profile")
                    .unwrap();
            }
        }
    }
}
