#[cfg(unix)]
use crate::scenario_steps;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
#[cfg(unix)]
#[cfg(any(target_os = "linux", target_os = "macos"))]
use keyringd_system_tests::application::{ApplicationConfiguration, SystemTestApplication};
#[cfg(unix)]
use keyringd_system_tests::authenticator::ApprovalMode;
#[cfg(unix)]
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
#[cfg(unix)]
use keyringd_system_tests::environment::SystemTestEnvironment;
#[cfg(unix)]
use keyringd_test_usbip::authenticator_server::client::AuthenticatorProfile;

use keyringd_testkit::scenario;

scenario! {
    group "Profile Recovery / Multiple Security Keys" {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        case "Fails recovery after UV when the selected USB key has no matching Passkey" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available() => "requires direct CTAP credential recovery";
            given "Two keys with only the first passkey and the new PC status" {
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
                scenario_steps::profile_setup::profile_fixture::create_recovery_source(
                    &mut environment,
                    &key,
                    "source-passkey",
                )
                .unwrap();
                let other_key = environment
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                environment.application().reset_profiles().unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
            }
            when "Select the second empty one and complete the UV" {
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                other_key.authenticator()
                    .reset(AuthenticatorProfile::UvConfiguredPinUnconfigured)
                    .unwrap();
                other_key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                other_key.connect().unwrap();
                scenario_steps::profile_setup::recovery_selection::discover_on_selected_key(
                    &mut environment,
                    &other_key,
                    &key,
                    "selected-empty-key",
                )
                .unwrap();
                environment
                    .application()
                    .capture_screenshot("Selected key has no Passkey")
                    .unwrap();
            }
            then "Do not wait for the first run and not save the Profile" {
                environment
                    .application()
                    .capture_screenshot("Selected key has no Passkey")
                    .unwrap();
                assert!(
                    environment
                        .application()
                        .stored_profiles()
                        .unwrap()
                        .is_empty()
                );
            }
            when "Return to the form with the existing Passkey selection and input in the back and cancel" {
                environment
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .back()
                    .unwrap();
                scenario_steps::profile_setup::form::assert_content(
                    &mut environment,
                    "selected-empty-key",
                    true,
                )
                .unwrap();
                scenario_steps::profile_setup::navigation::cancel(&mut environment).unwrap();
                environment.application().restart().unwrap();
            }
            then "Profile is not saved even after restarting" {
                scenario_steps::profile_management::profile_list::assert_absent(
                    &environment.application().profiles_window().unwrap(),
                    "selected-empty-key",
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
            when "Reopen and fail with an empty key, this time cancel directly from the absence screen" {
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                other_key.authenticator()
                    .reset(AuthenticatorProfile::UvConfiguredPinUnconfigured)
                    .unwrap();
                other_key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                other_key.connect().unwrap();
                scenario_steps::profile_setup::recovery_selection::discover_on_selected_key(
                    &mut environment,
                    &other_key,
                    &key,
                    "cancel-empty-key",
                )
                .unwrap();
                scenario_steps::profile_setup::navigation::cancel(&mut environment).unwrap();
            }
            then "Does not add profile even if you close the absence screen" {
                scenario_steps::profile_management::profile_list::assert_absent(
                    &environment.application().profiles_window().unwrap(),
                    "cancel-empty-key",
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
}
