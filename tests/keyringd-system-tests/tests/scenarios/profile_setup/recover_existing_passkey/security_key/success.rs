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
use keyringd_test_usbip::authenticator_server::client::{
    AuthenticatorConfiguration, LargeBlobSupport,
};
use keyringd_testkit::agent::AgentIdentity;
use keyringd_testkit::{evidence_log, evidence_text, scenario};

scenario! {
    group "Profile Recovery / Security Key" {
        case "Authorizes a new Device Key with the existing Passkey and keeps the same CA" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and the OS Security Key environment";
            given "Profile created on the old PC and Passkey on Security Key" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let key = environment
                    .authenticators()
                    .create_security_key(AuthenticatorConfiguration {
                        large_blob_support: LargeBlobSupport::LargeBlobKey,
                        ..Default::default()
                    })
                    .unwrap();
                let configuration = ApplicationConfiguration::default();
                environment.application().start(configuration).unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                scenario_steps::profile_setup::form::submit_creation(&mut environment, "source-pc")
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
                let source = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    "source-pc",
                )
                .unwrap();
                let source_attestation = environment.application().stored_profiles().unwrap()[0]
                    ["passkey"]["attestationResponse"].clone();
                assert!(source_attestation.is_object());
                let previous_requests = key.authenticator().request_count().unwrap();
                assert!(
                    previous_requests > 0,
                    "the key did not create the original passkey"
                );
            }
            when "Maintain keys and initialize save destination to new PC" {
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
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                scenario_steps::profile_setup::form::submit_recovery(
                    &mut environment,
                    "recovered-pc",
                )
                .unwrap();
                scenario_steps::profile_setup::transport_choices::assert_recovery(&mut environment)
                    .unwrap();
                environment
                    .application()
                    .capture_screenshot("Find existing Passkey transport choices")
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
            }
            when "Approve Passkey exploration with Security Key" {
                let requests =
                    scenario_steps::profile_setup::security_key::approve_step_with_presence(
                        &mut environment,
                        &key,
                        ProfileSetupStep::AuthorizeRecoveredDevice,
                    );
                evidence_log!("Passkey discovery requests", format!("{requests:?}"));
            }
            then "Continue to Device Key Authorization for New PC" {
                scenario_steps::profile_setup::progress::wait_for_step(&mut environment, keyringd_system_tests::application::features::profile_setup::ProfileSetupStep::AuthorizeRecoveredDevice).unwrap();
                scenario_steps::profile_setup::security_key::capture_prompt(
                    &mut environment,
                    "Recovered Device Key authorization prompt",
                )
                .unwrap();
            }
            when "Approve Device Key authorization with the same key" {
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
                    .capture_screenshot("Recovered profile")
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
            then "Only one original CA, Passkey and a new Device Key are saved" {
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "recovered-pc",
                )
                .unwrap();
                let stored = environment.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "recovered-pc");
                assert_eq!(stored[0]["passkey"]["attestationResponse"], source_attestation);
                let recovered = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    "recovered-pc",
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
                assert!(key.authenticator().request_count().unwrap() > previous_requests);
                evidence_text!("Unchanged Certificate Authority", &recovered.ca);
                evidence_text!("Source Device Key", &source.device);
                evidence_text!("Recovered Device Key", &recovered.device);
            }
            when "Connect SSH to a server that trusts only the original CA" {
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| {
                        identity.comment == "keyringd recovered-pc device certificate"
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
            then "You can authenticate with the new PC's Certificate without changing the server settings" {
                connection.assert_authenticated(&identity);
                evidence_log!("Recovered Device Certificate SSH", &connection.stderr);
                evidence_log!("sshd authentication log", &connection.server_log);
            }
        }
    }
}
