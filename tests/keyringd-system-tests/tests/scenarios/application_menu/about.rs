use crate::scenario_steps;
use keyringd_system_tests::application::ApplicationConfiguration;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_testkit::{evidence_text, scenario};

use keyringd_system_tests::application::features::about::{
    AboutWindowDriver, LicenseDocumentDriver, LicenseWindowDriver,
};
use keyringd_system_tests::environment::NativeSystemTestEnvironment;

scenario! {
    group "Application Menu / Navigation" {
        case "Opens About from the main menu" {
            given "Launch the application" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                environment
                    .application()
                    .start(ApplicationConfiguration::default())
                    .unwrap();
            }
            when "Select About Keyringd from the main menu" {
                environment.application().about_window().open().unwrap();
                environment
                    .application()
                    .about_window()
                    .capture_screenshot("About Keyringd")
                    .unwrap();
            }
            then "You can check the app name and product version" {
                scenario_steps::about::application_info::assert_content(
                    &environment.application().about_window(),
                )
                .unwrap();
            }
            when "Open application license" {
                let text = {
                    let mut about_window = environment.application().about_window();
                    let license = about_window.open_application_license().unwrap();
                    let text = license.text().unwrap();
                    license.capture_screenshot("Application license").unwrap();
                    license.close().unwrap();
                    text
                };
            }
            then "Check Apache licenses" {
                for clause in [
                    "Apache License",
                    "Version 2.0",
                    "END OF TERMS AND CONDITIONS",
                ] {
                    assert!(text.contains(clause), "missing license clause: {clause}");
                }
                evidence_text!("Application license", &text);
            }
            when "Open a third party license" {
                let text = {
                    let mut about_window = environment.application().about_window();
                    let license = about_window.open_third_party_licenses().unwrap();
                    let text = license.text().unwrap();
                    license.capture_screenshot("Third-party licenses").unwrap();
                    license.close().unwrap();
                    text
                };
            }
            then "You can check the dependency and license text of the target OS" {
                scenario_steps::about::third_party_licenses::assert_content(&text);
                evidence_text!("Third-party licenses", &text);
                environment.application().about_window().close().unwrap();
            }
            when "Open About again after closing" {
                environment.application().about_window().open().unwrap();
                environment
                    .application()
                    .about_window()
                    .capture_screenshot("Reopened About")
                    .unwrap();
            }
            then "App name and product version are also displayed on redisplayed About" {
                scenario_steps::about::application_info::assert_content(
                    &environment.application().about_window(),
                )
                .unwrap();
                environment.application().about_window().close().unwrap();
            }
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        case "Opens About from the background menu after closing the window" {
            given "Launch the app and close the profile list" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                environment
                    .application()
                    .start(ApplicationConfiguration::default())
                    .unwrap();
                environment
                    .application()
                    .profiles_window()
                    .unwrap()
                    .close()
                    .unwrap();
            }
            when "Select about keyringd from the background menu" {
                environment
                    .application()
                    .about_window()
                    .open_from_background_menu()
                    .unwrap();
                environment
                    .application()
                    .about_window()
                    .capture_screenshot("About from the background menu")
                    .unwrap();
            }
            then "App name and product version appear in the About panel" {
                scenario_steps::about::application_info::assert_content(
                    &environment.application().about_window(),
                )
                .unwrap();
                environment.application().about_window().close().unwrap();
            }
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        case "Exits from the background menu after closing About" {
            given "Launch the app" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                environment
                    .application()
                    .start(ApplicationConfiguration::default())
                    .unwrap();
            }
            when "Open and close About" {
                environment.application().about_window().open().unwrap();
                environment.application().about_window().close().unwrap();
            }
            when "Select End from the Background Menu" {
                environment.application().request_exit().unwrap();
            }
            then "App process ends successfully" {
                environment.application().wait_exited().unwrap();
            }
        }
    }
}
