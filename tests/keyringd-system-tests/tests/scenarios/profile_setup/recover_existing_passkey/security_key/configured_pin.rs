use keyringd_system_tests::application::ApplicationConfiguration;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
};
use keyringd_system_tests::application::features::profile_setup::{
    ProfileSetupStep, ProfileSetupWizardDriver,
};

use crate::scenario_steps;
use keyringd_system_tests::authenticator::ApprovalMode;
use keyringd_system_tests::environment::{NativeSystemTestEnvironment, SystemTestEnvironment};
use keyringd_system_tests::ssh::{SshServers, SshSession, SshTestServer};
use keyringd_test_usbip::authenticator_server::client::{
    AuthenticatorConfiguration, AuthenticatorProfile, LargeBlobSupport,
};
use keyringd_testkit::{evidence_log, scenario};

scenario! {
    group "Profile Recovery / Security Key with PIN" {
        case "Recovers on a new computer with a PIN-protected USB key and authenticates over SSH" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available() => "requires the native Security Key environment";
            given "Profile and public key of the original PC created with a PIN key" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let key = environment
                    .authenticators()
                    .create_security_key(AuthenticatorConfiguration {
                        large_blob_support: LargeBlobSupport::Unsupported,
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
                key.authenticator().reset(AuthenticatorProfile::UvUnconfiguredPinConfigured)
                    .unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                scenario_steps::profile_setup::form::submit_creation(
                    &mut environment,
                    "pin-source",
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
                let mut source_pin_entered = false;
                for next in [
                    ProfileSetupStep::AuthorizeNewDevice,
                    ProfileSetupStep::Completed,
                ] {
                    source_pin_entered |=
                        scenario_steps::profile_setup::security_key::approve_step_with_pin(
                            &mut environment,
                            &key,
                            "1234",
                            next,
                        )
                        .unwrap();
                    if !matches!(next, ProfileSetupStep::Completed) {
                        environment
                            .application()
                            .profile_setup_window()
                            .capture_screenshot(next.title())
                            .unwrap();
                    }
                }
                scenario_steps::profile_setup::security_key::wait_for_pin_closed(&mut environment).unwrap();
                scenario_steps::profile_setup::completion::wait_for(&mut environment).unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .capture_screenshot("Source profile created")
                    .unwrap();
                assert!(
                    source_pin_entered,
                    "source creation must exercise PIN input before recovery can use a cached PIN"
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
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "pin-source",
                )
                .unwrap();
                let original = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    "pin-source",
                )
                .unwrap();
                let previous_requests = key.authenticator().request_count().unwrap();
            }
            when "Maintain the same key and initialize the save destination to a new PC" {
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
                    "pin-recovered",
                )
                .unwrap();
                scenario_steps::profile_setup::transport_choices::assert_recovery(&mut environment)
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
                let mut recovery_pin_entered = false;
            }
            when "Explore/Passkey Validation with PIN Key Approve Device Key Authorization for New PC" {
                for next in [
                    ProfileSetupStep::RecoverPasskeyPublicKey,
                    ProfileSetupStep::AuthorizeRecoveredDevice,
                    ProfileSetupStep::Completed,
                ] {
                    recovery_pin_entered |=
                        scenario_steps::profile_setup::security_key::approve_step_with_pin(
                            &mut environment,
                            &key,
                            "1234",
                            next,
                        )
                        .unwrap();
                    if !matches!(next, ProfileSetupStep::Completed) {
                        environment
                            .application()
                            .profile_setup_window()
                            .capture_screenshot(next.title())
                            .unwrap();
                    }
                }
            }
            then "The profile with the same CA, Passkey and the new Device Key is saved as the original" {
                scenario_steps::profile_setup::security_key::wait_for_pin_closed(&mut environment).unwrap();
                scenario_steps::profile_setup::completion::wait_for(&mut environment).unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .capture_screenshot("Recovered profile created")
                    .unwrap();
                // Windows may reuse the PIN verified during source creation.
                evidence_log!(
                    "PIN entry during recovery",
                    format!("entered_pin={recovery_pin_entered}")
                );
                key.authenticator().set_approval_mode(ApprovalMode::Automatic).unwrap();
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
                    "pin-recovered",
                )
                .unwrap();
                let recovered = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    "pin-recovered",
                )
                .unwrap();
                assert_eq!(recovered.ca, original.ca);
                assert_eq!(
                    keyringd_testkit::agent::AgentIdentity::from_openssh(&recovered.passkey)
                        .unwrap()
                        .public_key,
                    keyringd_testkit::agent::AgentIdentity::from_openssh(&original.passkey)
                        .unwrap()
                        .public_key,
                );
                assert_ne!(recovered.device, original.device);
                assert!(key.authenticator().request_count().unwrap() > previous_requests);
                let stored = environment.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "pin-recovered");
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| {
                        identity.comment == "keyringd pin-recovered device certificate"
                    })
                    .unwrap();
            }
            when "Real SSH authentication from a new PC to a server with CA settings of the original PC" {
                let connection_server = environment.ssh_servers().start(&original.ca).unwrap();
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
            then "The original CA accepts the new PC's certificate and the SSH command succeeds" {
                connection.assert_authenticated(&identity);
                evidence_log!("Recovered PIN-protected USB profile SSH", connection.stderr);
                evidence_log!("sshd authentication log", connection.server_log);
            }
        }
    }
}
