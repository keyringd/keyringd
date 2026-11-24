use keyringd_system_tests::application::ApplicationConfiguration;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileFormPageDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;

use crate::scenario_steps;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_testkit::scenario;

scenario! {
    case "Returns to Profiles when canceled before setup starts" {
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
        when "Enter a name" {
            environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .profile_form_page()
                .unwrap()
                .enter_profile_name("not-created")
                .unwrap();
        }
        then "CREATE button is enabled" {
            scenario_steps::profile_setup::form::assert_create_enabled(&mut environment, true)
                .unwrap();
        }
        when "Close the wizard in Cancel and go back to the list" {
            scenario_steps::profile_setup::navigation::cancel(&mut environment).unwrap();
            environment
                .application()
                .capture_screenshot("Profiles after setup cancellation")
                .unwrap();
        }
        then "No profile is saved and no requests to the authenticator occur" {
            scenario_steps::profile_management::profile_list::assert_absent(
                &environment.application().profiles_window().unwrap(),
                "not-created",
            )
            .unwrap();
            assert!(
                environment
                    .application()
                    .stored_profiles()
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(key.authenticator().request_count().unwrap(), 0);
        }
    }
}
