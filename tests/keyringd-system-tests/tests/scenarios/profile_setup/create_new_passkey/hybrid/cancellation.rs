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
    case "Leaves no profile when canceled from the Hybrid QR screen" {
        skip_if: !keyringd_system_tests::authenticator::Authenticators::hybrid_available() => "Requires a virtual phone and a Hybrid test tunnel";
        given "Product creation wizard using virtual phones and empty destinations" {
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
        when "Start creating a new Passkey" {
            scenario_steps::profile_setup::form::submit_creation(
                &mut environment,
                "cancelled-hybrid",
            )
            .unwrap();
        }
        then "Shows how to create a new Passkey and how to actually read it" {
            scenario_steps::profile_setup::transport_choices::assert_creation(&mut environment)
                .unwrap();
            let (_, image) = environment
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
            evidence_image!("Hybrid QR before cancellation", image);
        }
        when "Close the wizard in Cancel and wait for the end of the request" {
            scenario_steps::profile_setup::navigation::cancel(&mut environment).unwrap();
            environment
                .application()
                .capture_screenshot("Profiles after setup cancellation")
                .unwrap();
        }
        then "The profile in the middle does not remain in the list or the save destination" {
            scenario_steps::profile_management::profile_list::assert_absent(
                &environment.application().profiles_window().unwrap(),
                "cancelled-hybrid",
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
        when "Try creating a virtual phone again with the same app" {
            environment
                .application()
                .profile_setup_window()
                .open()
                .unwrap();
            scenario_steps::profile_setup::hybrid_profiles::create(
                &mut environment,
                &phone,
                "after-hybrid-cancel",
            )
            .unwrap();
            environment
                .application()
                .capture_screenshot("Profile setup result")
                .unwrap();
        }
        then "Only one case is saved without being disturbed by the previous operation" {
            let stored = environment.application().stored_profiles().unwrap();
            assert_eq!(stored.len(), 1);
            assert_eq!(stored[0]["displayName"], "after-hybrid-cancel");
        }
    }
}
