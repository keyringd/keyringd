use keyringd_system_tests::application::ApplicationConfiguration;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileFormPageDriver, ProfileSetupInteractionPageDriver,
    ProfileSetupQrViewDriver,
};

use crate::scenario_steps;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_testkit::scenario;

scenario! {
    case "Trims the profile name and displays a single saved profile" {
        given "Create a product wizard that has been opened from an empty location" {
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
        }
        when "Enter a name with a space to open advanced settings" {
            environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .profile_form_page()
                .unwrap()
                .enter_profile_name(" Work ")
                .unwrap();
            environment
                .application()
                .capture_screenshot("Profile setup page")
                .unwrap();
            environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .profile_form_page()
                .unwrap()
                .show_advanced_settings()
                .unwrap();
            environment
                .application()
                .capture_screenshot("Profile advanced settings")
                .unwrap();
            scenario_steps::profile_setup::form::submit_creation(&mut environment, " Work ")
                .unwrap();
        }
        then "Shows how to create a new Passkey" {
            scenario_steps::profile_setup::transport_choices::assert_creation(&mut environment)
                .unwrap();
        }
        when "Select and Approve Security Key" {
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
        then "Profile name and available operations are displayed on the completion screen" {
            let completed = environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .completion_page()
                .unwrap()
                .content()
                .unwrap();
            assert!(completed.messages.iter().any(|text| text.contains("Work")), "{completed:?}");
            assert!(completed.public_keys_enabled && completed.done_enabled, "{completed:?}");
            environment
                .application()
                .capture_screenshot("Setup completed")
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
        then "There is only one normalized name in the list and saved contents." {
            scenario_steps::profile_management::profile_list::assert_visible(
                &environment.application().profiles_window().unwrap(),
                "Work",
            )
            .unwrap();
            scenario_steps::profile_management::profile_list::assert_absent(
                &environment.application().profiles_window().unwrap(),
                " Work ",
            )
            .unwrap();
            let stored = environment.application().stored_profiles().unwrap();
            assert_eq!(stored.len(), 1);
            assert_eq!(stored[0]["displayName"], "Work");
            environment
                .application()
                .capture_screenshot("Created normalized profile")
                .unwrap();
        }
    }
}
