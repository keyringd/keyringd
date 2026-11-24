use crate::scenario_steps;
use keyringd_system_tests::application::ApplicationConfiguration;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileFormPageDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_testkit::scenario;

scenario! {
    case "Prevents creation with an empty or blank profile name" {
        given "Create a product wizard that has been opened from an empty location" {
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
        }
        then "CREATE button is invalid for empty name" {
            scenario_steps::profile_setup::form::assert_create_enabled(&mut environment, false)
                .unwrap();
        }
        when "Enter a valid name" {
            environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .profile_form_page()
                .unwrap()
                .enter_profile_name("Work")
                .unwrap();
        }
        then "The Create button is valid and opens the advanced settings" {
            scenario_steps::profile_setup::form::assert_create_enabled(&mut environment, true)
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
        }
        when "Replace it with a blank name only" {
            environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .profile_form_page()
                .unwrap()
                .enter_profile_name("   ")
                .unwrap();
        }
        then "Creation can not be started, no requests or saves from the authenticator" {
            scenario_steps::profile_setup::form::assert_create_enabled(&mut environment, false)
                .unwrap();
            assert_eq!(key.authenticator().request_count().unwrap(), 0);
            assert!(
                environment
                    .application()
                    .stored_profiles()
                    .unwrap()
                    .is_empty()
            );
            environment
                .application()
                .capture_screenshot("Whitespace-only profile name")
                .unwrap();
        }
    }

    case "Returns to editing when the profile name exceeds 80 characters" {
        given "Create a product wizard that has been opened from an empty location" {
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
            let name = "p".repeat(81);
        }
        when "Start creation with 81 character name" {
            environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .profile_form_page()
                .unwrap()
                .enter_profile_name(&name)
                .unwrap();
            environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .profile_form_page()
                .unwrap()
                .submit()
                .unwrap();
        }
        then "Return to the edit screen with original input and selection from error" {
            scenario_steps::profile_setup::form::assert_error(
                &mut environment,
                "Enter a profile name with 1 to 80 characters, without surrounding whitespace, line breaks, or control characters.",
            )
            .unwrap();
            scenario_steps::profile_setup::form::assert_create_enabled(&mut environment, true)
                .unwrap();
            scenario_steps::profile_setup::form::assert_content(&mut environment, &name, false)
                .unwrap();
            assert!(
                environment
                    .application()
                    .stored_profiles()
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(key.authenticator().request_count().unwrap(), 0);
            environment
                .application()
                .capture_screenshot("Profile name length validation")
                .unwrap();
        }
    }
}
