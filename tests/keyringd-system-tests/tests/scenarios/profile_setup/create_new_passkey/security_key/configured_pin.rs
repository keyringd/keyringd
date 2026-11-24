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
use keyringd_test_usbip::authenticator_server::client::AuthenticatorProfile;
use keyringd_testkit::{evidence_log, scenario};

scenario! {
    group "Profile Setup / Security Key with PIN" {
        case "Creates a profile with a PIN-protected USB key and authenticates over SSH" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available() => "requires the native Security Key environment";
            given "Internal UV-free PIN-configured USB key and empty profile" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let key = environment
                    .authenticators()
                    .create_security_key(Default::default())
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
            }
            when "Start creating a profile and select the USB key" {
                scenario_steps::profile_setup::form::submit_creation(
                    &mut environment,
                    "pin-created",
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
                let mut pin_entered = false;
            }
            when "Approve Passkey creation and Device Key authorization on your PIN" {
                for next in [
                    ProfileSetupStep::AuthorizeNewDevice,
                    ProfileSetupStep::Completed,
                ] {
                    pin_entered |=
                        scenario_steps::profile_setup::security_key::approve_step_with_pin(
                            &mut environment,
                            &key,
                            "1234",
                            next,
                        )
                        .unwrap();
                    if matches!(next, ProfileSetupStep::Completed) {
                        scenario_steps::profile_setup::security_key::wait_for_pin_closed(&mut environment).unwrap();
                        scenario_steps::profile_setup::completion::wait_for(&mut environment).unwrap();
                    }
                    environment
                        .application()
                        .profile_setup_window()
                        .capture_screenshot(next.title())
                        .unwrap();
                }
            }
            then "One profile is saved after the actual PIN input" {
                assert!(
                    pin_entered,
                    "the PIN-protected key completed without exercising PIN input"
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
                    "pin-created",
                )
                .unwrap();
                let stored = environment.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "pin-created");
                assert!(key.authenticator().request_count().unwrap() > 0);
                let ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut environment,
                    "pin-created",
                )
                .unwrap();
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| {
                        identity.comment == "keyringd pin-created device certificate"
                    })
                    .unwrap();
            }
            when "Real SSH authentication with the certificate of the created profile" {
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
            then "Certificate accepted and the ssh command succeeds" {
                connection.assert_authenticated(&identity);
                evidence_log!("PIN-protected USB profile SSH", connection.stderr);
                evidence_log!("sshd authentication log", connection.server_log);
            }
        }
    }
}
