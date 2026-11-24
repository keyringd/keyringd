use keyringd_system_tests::application::features::profile_setup::ProfileSetupStep;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
};

use crate::scenario_steps;
use keyringd_system_tests::application::{ApplicationConfiguration, SystemTestApplication};
use keyringd_system_tests::authenticator::ApprovalMode;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_system_tests::ssh::{SshServers, SshSession, SshTestServer};
use keyringd_testkit::agent::AgentIdentity;
use keyringd_testkit::{evidence_log, scenario};

scenario! {
    group "Profile Setup / Multiple Security Keys" {
        case "Creates and recovers a profile using only the selected USB key" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and the OS Security Key environment";
            given "Connect 2 separate manual-approved Security Keys" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let other_key = environment
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                other_key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
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
            when "Start creating a new Passkey and choose a second key" {
                scenario_steps::profile_setup::form::submit_creation(
                    &mut environment,
                    "selected-key",
                )
                .unwrap();
                scenario_steps::profile_setup::transport_choices::assert_creation(&mut environment)
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
                scenario_steps::profile_setup::security_key::wait_for_selection(&mut environment)
                    .unwrap();
                scenario_steps::profile_setup::security_key::capture_prompt(
                    &mut environment,
                    "Choose a Security Key",
                )
                .unwrap();
                scenario_steps::profile_setup::security_key::select(
                    &mut environment,
                    &key,
                    &other_key,
                )
                .unwrap();
                let requests =
                    scenario_steps::profile_setup::security_key::approve_step_with_presence(
                        &mut environment,
                        &key,
                        ProfileSetupStep::AuthorizeNewDevice,
                    );
                evidence_log!("Selected key creation requests", format!("{requests:?}"));
            }
            then "Device Key authorization continues with the same key, and does not approve any unselected key operations" {
                scenario_steps::profile_setup::progress::wait_for_step(&mut environment, keyringd_system_tests::application::features::profile_setup::ProfileSetupStep::AuthorizeNewDevice).unwrap();
                let request = scenario_steps::profile_setup::security_key::wait_for_request(
                    &mut environment,
                    &key,
                )
                .unwrap();
                assert!(request.user_presence_required);
                let other_requests = other_key.authenticator().pending_requests().unwrap();
                evidence_log!(
                    "Unapproved probes on the other key",
                    format!("{other_requests:?}")
                );
                scenario_steps::profile_setup::security_key::capture_prompt(
                    &mut environment,
                    "Device Key authorization prompt for the selected key",
                )
                .unwrap();
            }
            when "Approve Device Key authorization with the selected key" {
                let requests =
                    scenario_steps::profile_setup::security_key::approve_step_with_presence(
                        &mut environment,
                        &key,
                        ProfileSetupStep::Completed,
                    );
                evidence_log!(
                    "Selected key authorization requests",
                    format!("{requests:?}")
                );
                other_key.authenticator().wait_for_no_pending_requests().unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Automatic).unwrap();
                scenario_steps::profile_setup::completion::wait_for(&mut environment).unwrap();
                environment
                    .application()
                    .capture_screenshot("Profile created with the second key")
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
                    "selected-key",
                )
                .unwrap();
                let source = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    "selected-key",
                )
                .unwrap();
            }
            when "Restores to a new empty PC with only the keys you select with other keys unavailable" {
                other_key.disconnect().unwrap();
                environment.application().reset_profiles().unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                assert!(
                    environment
                        .application()
                        .stored_profiles()
                        .unwrap()
                        .is_empty()
                );
                scenario_steps::profile_setup::form::submit_recovery(
                    &mut environment,
                    "recovered-selected-key",
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
            }
            then "Preserve Passkey and CA of the selected keys to make independent Device Key" {
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "recovered-selected-key",
                )
                .unwrap();
                assert_eq!(
                    environment.application().stored_profiles().unwrap().len(),
                    1
                );
                let recovered = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    "recovered-selected-key",
                )
                .unwrap();
                assert_eq!(recovered.ca, source.ca);
                assert_eq!(
                    AgentIdentity::from_openssh(&recovered.passkey)
                        .unwrap()
                        .public_key,
                    AgentIdentity::from_openssh(&source.passkey)
                        .unwrap()
                        .public_key
                );
                assert_ne!(
                    AgentIdentity::from_openssh(&recovered.device)
                        .unwrap()
                        .public_key,
                    AgentIdentity::from_openssh(&source.device)
                        .unwrap()
                        .public_key
                );
            }
            when "Connect SSH to a server that trusts only the original CA" {
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| {
                        identity.comment == "keyringd recovered-selected-key device certificate"
                    })
                    .unwrap();
                let connection_server = environment.ssh_servers().start(&source.ca).unwrap();
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
            then "Real SSH authenticated with a new Certificate that is associated with the selected key" {
                connection.assert_authenticated(&identity);
                evidence_log!("Recovered selected key SSH", &connection.stderr);
                evidence_log!("sshd authentication log", &connection.server_log);
            }
        }
    }
}
