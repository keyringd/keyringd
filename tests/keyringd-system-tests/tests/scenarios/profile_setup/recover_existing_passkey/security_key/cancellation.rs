use keyringd_system_tests::application::ApplicationConfiguration;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
};

use crate::scenario_steps;
use keyringd_system_tests::authenticator::ApprovalMode;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_testkit::{evidence_text, scenario};

scenario! {
    group "Profile Recovery / Security Key / Passkey Discovery" {
        case "Leaves no profile when Security Key Passkey discovery is canceled" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and the OS Security Key environment";
            given "Empty Destination and Manually Approved Security Key" {
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
                scenario_steps::profile_setup::form::submit_creation(&mut environment, "source")
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
                environment.application().reset_profiles().unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
            }
            when "Start an existing Passkey exploration and select a Security Key" {
                scenario_steps::profile_setup::form::submit_recovery(
                    &mut environment,
                    "cancel-security-key-recovery",
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
                let request = scenario_steps::profile_setup::security_key::wait_for_request(
                    &mut environment,
                    &key,
                )
                .unwrap();
            }
            then "The actual Security Key is waiting for the user's operation" {
                assert!(request.user_presence_required, "{request:?}");
                evidence_text!("Pending Security Key request", format!("{request:?}"));
                scenario_steps::profile_setup::security_key::capture_prompt(
                    &mut environment,
                    "Security Key approval pending",
                )
                .unwrap();
            }
            when "Cancel from the product screen" {
                scenario_steps::profile_setup::navigation::cancel(&mut environment).unwrap();
                environment
                    .application()
                    .capture_screenshot("Profiles after setup cancellation")
                    .unwrap();
            }
            then "The request and operation screen are closed and the profile is not saved or published." {
                scenario_steps::profile_setup::security_key::assert_operation_finished(
                    &mut environment,
                    &key,
                )
                .unwrap();
                scenario_steps::profile_management::profile_list::assert_absent(
                    &environment.application().profiles_window().unwrap(),
                    "cancel-security-key-recovery",
                )
                .unwrap();
                assert!(
                    environment
                        .application()
                        .stored_profiles()
                        .unwrap()
                        .is_empty()
                );
                assert!(
                    environment
                        .application()
                        .agent_identities()
                        .unwrap()
                        .is_empty()
                );
            }
            when "The same app and key to operate again" {
                key.authenticator().set_approval_mode(ApprovalMode::Automatic).unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                scenario_steps::profile_setup::form::submit_recovery(
                    &mut environment,
                    "after-cancel",
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
            then "Only one case is created without being disturbed by the old demand" {
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "after-cancel",
                )
                .unwrap();
                let stored = environment.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "after-cancel");
                environment
                    .application()
                    .capture_screenshot("Profile after canceled operation")
                    .unwrap();
            }
        }
    }
}
