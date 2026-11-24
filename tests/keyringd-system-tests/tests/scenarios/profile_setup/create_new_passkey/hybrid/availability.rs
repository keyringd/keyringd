use keyringd_system_tests::application::features::profile_setup::ProfileSetupInteractionPageDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;

use crate::scenario_steps;
use keyringd_system_tests::application::{ApplicationConfiguration, SystemTestApplication};
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_testkit::scenario;

scenario! {
    group "Profile Setup / Hybrid Bluetooth Availability" {
        case "Explains disabled Bluetooth on the QR screen" {
            skip_if: !NativeSystemTestEnvironment::can_disable_bluetooth()
                => "requires controllable Bluetooth adapters on this OS";
            given "PCs that are not available with Bluetooth and where empty save destinations" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                #[cfg(target_os = "linux")]
                let bluetooth = environment.testbed().disable_bluetooth().unwrap();
                let configuration = ApplicationConfiguration::default();
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                let configuration = ApplicationConfiguration {
                    test_fido_mode: Some(
                        keyringd_core::test_support::TestFidoMode::BluetoothDisabled,
                    ),
                    ..configuration
                };
                environment.application().start(configuration).unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
            }
            when "Start creating or recovering a profile" {
                scenario_steps::profile_setup::form::submit_creation(
                    &mut environment,
                    "bluetooth-disabled",
                )
                .unwrap();
                scenario_steps::profile_setup::progress::wait_for_step(
                    &mut environment,
                    keyringd_system_tests::application::features::profile_setup::ProfileSetupStep::CreatePasskey,
                )
                .unwrap();
            }
            then "Show reasons why it is not available and how to switch to Security Key" {
                scenario_steps::profile_setup::transport_choices::assert_bluetooth_disabled(
                    &environment
                        .application()
                        .profile_setup_window()
                        .wizard()
                        .unwrap()
                        .interaction_page()
                        .unwrap()
                        .qr_view()
                        .unwrap(),
                )
                .unwrap();
                environment
                    .application()
                    .capture_screenshot("Bluetooth unavailable")
                    .unwrap();
            }
            when "Return to the input page and cancel" {
                scenario_steps::profile_setup::navigation::back_to_form(
                    &mut environment,
                    "bluetooth-disabled",
                    false,
                )
                .unwrap();
                scenario_steps::profile_setup::form::assert_content(
                    &mut environment,
                    "bluetooth-disabled",
                    false,
                )
                .unwrap();
                scenario_steps::profile_setup::navigation::cancel(&mut environment).unwrap();
                environment
                    .application()
                    .capture_screenshot("Profiles after setup cancellation")
                    .unwrap();
                #[cfg(target_os = "linux")]
                bluetooth.restore().unwrap();
            }
            then "The request has ended and the profile is not saved or published" {
                scenario_steps::profile_management::profile_list::assert_absent(
                    &environment.application().profiles_window().unwrap(),
                    "bluetooth-disabled",
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
        }
        case "Explains a missing Bluetooth adapter on the QR screen" {
            given "PCs that are not available with Bluetooth and where empty save destinations" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                #[cfg(target_os = "linux")]
                let bluetooth = environment.testbed().hide_bluetooth_adapters().unwrap();
                let configuration = ApplicationConfiguration::default();
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                let configuration = ApplicationConfiguration {
                    test_fido_mode: Some(
                        keyringd_core::test_support::TestFidoMode::NoBluetoothAdapter,
                    ),
                    ..configuration
                };
                environment.application().start(configuration).unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
            }
            when "Start creating or recovering a profile" {
                scenario_steps::profile_setup::form::submit_creation(
                    &mut environment,
                    "no-bluetooth-adapter",
                )
                .unwrap();
                scenario_steps::profile_setup::progress::wait_for_step(
                    &mut environment,
                    keyringd_system_tests::application::features::profile_setup::ProfileSetupStep::CreatePasskey,
                )
                .unwrap();
            }
            then "Show reasons why it is not available and how to switch to Security Key" {
                scenario_steps::profile_setup::transport_choices::assert_no_bluetooth_adapter(
                    &environment
                        .application()
                        .profile_setup_window()
                        .wizard()
                        .unwrap()
                        .interaction_page()
                        .unwrap()
                        .qr_view()
                        .unwrap(),
                )
                .unwrap();
                environment
                    .application()
                    .capture_screenshot("Bluetooth unavailable")
                    .unwrap();
            }
            when "Return to the input page and cancel" {
                scenario_steps::profile_setup::navigation::back_to_form(
                    &mut environment,
                    "no-bluetooth-adapter",
                    false,
                )
                .unwrap();
                scenario_steps::profile_setup::form::assert_content(
                    &mut environment,
                    "no-bluetooth-adapter",
                    false,
                )
                .unwrap();
                scenario_steps::profile_setup::navigation::cancel(&mut environment).unwrap();
                environment
                    .application()
                    .capture_screenshot("Profiles after setup cancellation")
                    .unwrap();
                #[cfg(target_os = "linux")]
                bluetooth.restore().unwrap();
            }
            then "The request has ended and the profile is not saved or published" {
                scenario_steps::profile_management::profile_list::assert_absent(
                    &environment.application().profiles_window().unwrap(),
                    "no-bluetooth-adapter",
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
        }
    }
}
