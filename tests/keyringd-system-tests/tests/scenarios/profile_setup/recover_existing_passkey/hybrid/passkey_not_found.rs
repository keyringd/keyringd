use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
use keyringd_system_tests::application::features::profile_setup::{
    ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
};

use crate::scenario_steps;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_testkit::{evidence_image, scenario};

scenario! {
    case "Explains when the phone or tablet has no matching Passkey" {
        skip_if: !keyringd_system_tests::authenticator::Authenticators::hybrid_available() => "Requires a virtual phone and a Hybrid test tunnel";
        given "Virtual Phones without Passkey and Empty Destinations" {
            let mut environment = NativeSystemTestEnvironment::new().unwrap();
            let phone = environment
                .authenticators()
                .create_hybrid_phone(Default::default())
                .unwrap();
            #[cfg(target_os = "linux")]
            let _advertisements = environment
                .testbed()
                .remote_hybrid_advertisements()
                .unwrap();
            let configuration = keyringd_system_tests::application::ApplicationConfiguration {
                #[cfg(any(target_os = "macos", windows))]
                accept_remote_hybrid_advertisements: true,
                ..Default::default()
            };
            environment.application().start(configuration).unwrap();
            environment
                .application()
                .profile_setup_window()
                .open()
                .unwrap();
        }
        when "Explore an existing Passkey and receive NoCredentials from a real phone" {
            scenario_steps::profile_setup::form::submit_recovery(
                &mut environment,
                "missing-phone-passkey",
            )
            .unwrap();
            scenario_steps::profile_setup::transport_choices::assert_recovery(&mut environment)
                .unwrap();
            let (qr, image) = environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .interaction_page()
                .unwrap()
                .qr_view()
                .unwrap()
                .read_hybrid_qr(None)
                .unwrap();
            evidence_image!("Find missing Passkey QR", image);
            let advertisement = phone.scan_hybrid_qr(&qr).unwrap();
            environment
                .application()
                .send_hybrid_advertisement(&advertisement)
                .unwrap();
        }
        then "Passkey is missing guide and continues button" {
            scenario_steps::profile_setup::passkey_not_found::assert_visible(&mut environment)
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
                .capture_screenshot("No Passkey on Phone")
                .unwrap();
        }
        when "Close the wizard with Cancel" {
            scenario_steps::profile_setup::navigation::cancel(&mut environment).unwrap();
            environment
                .application()
                .capture_screenshot("Profiles after setup cancellation")
                .unwrap();
        }
        then "Return to the profile list and nothing is saved" {
            scenario_steps::profile_management::profile_list::assert_absent(
                &environment.application().profiles_window().unwrap(),
                "missing-phone-passkey",
            )
            .unwrap();
            assert!(
                environment
                    .application()
                    .stored_profiles()
                    .unwrap()
                    .is_empty()
            );
        }
        when "Search again on the same phone and choose to switch to new creation" {
            environment
                .application()
                .profile_setup_window()
                .open()
                .unwrap();
            scenario_steps::profile_setup::form::submit_recovery(
                &mut environment,
                "missing-phone-passkey",
            )
            .unwrap();
            scenario_steps::profile_setup::transport_choices::assert_recovery(&mut environment)
                .unwrap();
            let (qr, _) = environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .interaction_page()
                .unwrap()
                .qr_view()
                .unwrap()
                .read_hybrid_qr(None)
                .unwrap();
            let advertisement = phone.scan_hybrid_qr(&qr).unwrap();
            environment
                .application()
                .send_hybrid_advertisement(&advertisement)
                .unwrap();
            scenario_steps::profile_setup::passkey_not_found::assert_visible(&mut environment)
                .unwrap();
            scenario_steps::profile_setup::passkey_not_found::start_new_creation(
                &mut environment,
                "missing-phone-passkey",
            )
            .unwrap();
        }
        then "A new creation is selected and the key is not created immediately after returning to the warning form." {
            scenario_steps::profile_setup::form::assert_content(
                &mut environment,
                "missing-phone-passkey",
                false,
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
            environment
                .application()
                .capture_screenshot("New Passkey selected after recovery failure")
                .unwrap();
        }
        when "Restart the app" {
            environment.application().restart().unwrap();
        }
        then "A failed recovery profile does not exist after restarting" {
            scenario_steps::profile_management::profile_list::assert_absent(
                &environment.application().profiles_window().unwrap(),
                "missing-phone-passkey",
            )
            .unwrap();
            assert!(
                environment
                    .application()
                    .stored_profiles()
                    .unwrap()
                    .is_empty()
            );
        }
    }
}
