use keyringd_system_tests::application::ApplicationConfiguration;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
};

use crate::scenario_steps;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_testkit::scenario;

scenario! {
    case "Retries a failed save in the same wizard" {
        given "Empty destinations and virtual security keys that only save fails" {
            let mut environment = NativeSystemTestEnvironment::new().unwrap();
            let _key = environment
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
            environment
                .application()
                .set_profile_save_failure(true)
                .unwrap();
        }
        when "Approve Passkey and Device Key" {
            scenario_steps::profile_setup::form::submit_creation(&mut environment, "Retry")
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
        }
        then "The reason for the save failure and the input screen that can be retried is displayed, and the key is not exposed to the agent." {
            scenario_steps::profile_setup::form::assert_error(&mut environment, "Could not save the profile changes. Check that the profile storage folder is writable and has available space, then try again.").unwrap();
            scenario_steps::profile_setup::form::assert_content(&mut environment, "Retry", false)
                .unwrap();
            assert!(
                environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .is_empty()
            );
            environment
                .application()
                .capture_screenshot("Profile save-error")
                .unwrap();
        }
        when "Restore the save destination" {
            environment
                .application()
                .set_profile_save_failure(false)
                .unwrap();
        }
        then "Failed profile not saved" {
            assert!(
                environment
                    .application()
                    .stored_profiles()
                    .unwrap()
                    .is_empty()
            );
        }
        when "Retry the creation with the same wizard" {
            scenario_steps::profile_setup::form::submit_creation(&mut environment, "Retry")
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
        }
        then "Profile operation is displayed" {
            scenario_steps::profile_setup::completion::wait_for(&mut environment).unwrap();
            environment
                .application()
                .capture_screenshot("Profile created after storage retry")
                .unwrap();
        }
        when "Return to the list from the completion screen" {
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
        then "Only one profile is saved and displayed in the list" {
            scenario_steps::profile_management::profile_list::assert_visible(
                &environment.application().profiles_window().unwrap(),
                "Retry",
            )
            .unwrap();
            let stored = environment.application().stored_profiles().unwrap();
            assert_eq!(stored.len(), 1);
            assert_eq!(stored[0]["displayName"], "Retry");
            environment
                .application()
                .capture_screenshot("Retry completed")
                .unwrap();
        }
    }
}
