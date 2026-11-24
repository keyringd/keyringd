#[cfg(unix)]
use crate::scenario_steps;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
};
#[cfg(unix)]
#[cfg(any(target_os = "linux", target_os = "macos"))]
use keyringd_system_tests::application::{ApplicationConfiguration, SystemTestApplication};
#[cfg(unix)]
use keyringd_system_tests::authenticator::ApprovalMode;
#[cfg(unix)]
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
#[cfg(unix)]
use keyringd_system_tests::environment::SystemTestEnvironment;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use keyringd_test_usbip::authenticator_server::client::AuthenticatorProfile;

use keyringd_testkit::scenario;

scenario! {
    group "Profile Recovery / Built-in User Verification" {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        case "Prefers built-in UV during recovery when UV and PIN are both configured" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#gettingPinUvAuthToken"
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and the OS Security Key environment";
            given "Prepare an existing Passkey on a UV-configured Security Key" {
                let mut driver = NativeSystemTestEnvironment::new().unwrap();
                let key = driver
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                let configuration = ApplicationConfiguration::default();
                driver.application().start(configuration).unwrap();
                driver.application().profile_setup_window().open().unwrap();
                key.authenticator().reset(AuthenticatorProfile::UvConfiguredPinConfigured)
                    .unwrap();
                scenario_steps::profile_setup::form::submit_creation(&mut driver, "source-pc")
                    .unwrap();
                driver
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
                driver
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
                let original_ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut driver,
                    "source-pc",
                )
                .unwrap();
                driver.application().reset_profiles().unwrap();
                driver.application().profile_setup_window().open().unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
            }
            when "Approve Search, Validate, and Device Key Authorization with Built-in UV" {
                scenario_steps::profile_setup::security_key_profiles::recover_with_built_in_uv(
                    &mut driver,
                    &key,
                    "recover-uv-pin",
                );
            }
            then "You can recover without requiring a PIN in both stages" {
                driver
                    .application()
                    .capture_screenshot("Profile setup result")
                    .unwrap();
                driver
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
                scenario_steps::profile_management::profile_list::assert_visible(
                    &driver.application().profiles_window().unwrap(),
                    "recover-uv-pin",
                )
                .unwrap();
                assert_eq!(driver.application().stored_profiles().unwrap().len(), 1);
                assert_eq!(
                    scenario_steps::profile_management::public_keys::copy_ca(
                        &mut driver,
                        "recover-uv-pin"
                    )
                    .unwrap(),
                    original_ca
                );
            }
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        case "Uses built-in UV during recovery without setting a PIN" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#gettingPinUvAuthToken"
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and the OS Security Key environment";
            given "Prepare an existing Passkey on a UV-configured Security Key" {
                let mut driver = NativeSystemTestEnvironment::new().unwrap();
                let key = driver
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                let configuration = ApplicationConfiguration::default();
                driver.application().start(configuration).unwrap();
                driver.application().profile_setup_window().open().unwrap();
                key.authenticator().reset(AuthenticatorProfile::UvConfiguredPinUnconfigured)
                    .unwrap();
                scenario_steps::profile_setup::form::submit_creation(&mut driver, "source-pc")
                    .unwrap();
                driver
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
                driver
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
                let original_ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut driver,
                    "source-pc",
                )
                .unwrap();
                driver.application().reset_profiles().unwrap();
                driver.application().profile_setup_window().open().unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
            }
            when "Approve Search, Validate, and Device Key Authorization with Built-in UV" {
                scenario_steps::profile_setup::security_key_profiles::recover_with_built_in_uv(
                    &mut driver,
                    &key,
                    "recover-uv-only",
                );
            }
            then "You can recover without requiring a PIN in both stages" {
                driver
                    .application()
                    .capture_screenshot("Profile setup result")
                    .unwrap();
                driver
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
                scenario_steps::profile_management::profile_list::assert_visible(
                    &driver.application().profiles_window().unwrap(),
                    "recover-uv-only",
                )
                .unwrap();
                assert_eq!(driver.application().stored_profiles().unwrap().len(), 1);
                assert_eq!(
                    scenario_steps::profile_management::public_keys::copy_ca(
                        &mut driver,
                        "recover-uv-only"
                    )
                    .unwrap(),
                    original_ca
                );
            }
        }
    }
}

scenario! {
    group "Profile Recovery / PIN" {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        case "Uses the configured PIN for each recovery operation when built-in UV is unavailable" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#gettingPinUvAuthToken"
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and direct CTAP Security Key support";
            given "Existing Passkey and empty new PCs protected by PIN" {
                let mut driver = NativeSystemTestEnvironment::new().unwrap();
                let key = driver
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                let configuration = ApplicationConfiguration::default();
                driver.application().start(configuration).unwrap();
                driver.application().profile_setup_window().open().unwrap();
                key.authenticator().reset(AuthenticatorProfile::UvUnconfiguredPinConfigured)
                    .unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                scenario_steps::profile_setup::security_key_profiles::create_with_pin(
                    &mut driver,
                    &key,
                    "source-pc",
                    false,
                );
                driver
                    .application()
                    .capture_screenshot("Profile setup result")
                    .unwrap();
                driver
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
                let original_ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut driver,
                    "source-pc",
                )
                .unwrap();
                driver.application().reset_profiles().unwrap();
                driver.application().profile_setup_window().open().unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
            }
            when "Enter the configured PIN with Search/Verification/Device Key Authorization" {
                scenario_steps::profile_setup::security_key_profiles::recover_with_pin(
                    &mut driver,
                    &key,
                    "recover-configured-pin",
                );
            }
            then "You can recover the profile of the same CA with a PIN for each operation" {
                driver
                    .application()
                    .capture_screenshot("Profile setup result")
                    .unwrap();
                driver
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
                scenario_steps::profile_management::profile_list::assert_visible(
                    &driver.application().profiles_window().unwrap(),
                    "recover-configured-pin",
                )
                .unwrap();
                let stored = driver.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "recover-configured-pin");
                assert_eq!(
                    scenario_steps::profile_management::public_keys::copy_ca(
                        &mut driver,
                        "recover-configured-pin"
                    )
                    .unwrap(),
                    original_ca
                );
            }
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        case "Recovers a Passkey created before PIN or UV setup using the configured PIN" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#settingNewPin"
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and direct CTAP Security Key support";
            given "Existing Passkey and empty new PCs protected by PIN" {
                let mut driver = NativeSystemTestEnvironment::new().unwrap();
                let key = driver
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                let configuration = ApplicationConfiguration::default();
                driver.application().start(configuration).unwrap();
                driver.application().profile_setup_window().open().unwrap();
                key.authenticator().reset(AuthenticatorProfile::UvUnconfiguredPinUnconfigured)
                    .unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                scenario_steps::profile_setup::security_key_profiles::create_with_pin(
                    &mut driver,
                    &key,
                    "source-pc",
                    true,
                );
                driver
                    .application()
                    .capture_screenshot("Profile setup result")
                    .unwrap();
                driver
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
                let original_ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut driver,
                    "source-pc",
                )
                .unwrap();
                driver.application().reset_profiles().unwrap();
                driver.application().profile_setup_window().open().unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
            }
            when "Enter the configured PIN with Search/Verification/Device Key Authorization" {
                scenario_steps::profile_setup::security_key_profiles::recover_with_pin(
                    &mut driver,
                    &key,
                    "recover-new-pin",
                );
            }
            then "You can recover the profile of the same CA with a PIN for each operation" {
                driver
                    .application()
                    .capture_screenshot("Profile setup result")
                    .unwrap();
                driver
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
                scenario_steps::profile_management::profile_list::assert_visible(
                    &driver.application().profiles_window().unwrap(),
                    "recover-new-pin",
                )
                .unwrap();
                let stored = driver.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "recover-new-pin");
                assert_eq!(
                    scenario_steps::profile_management::public_keys::copy_ca(
                        &mut driver,
                        "recover-new-pin"
                    )
                    .unwrap(),
                    original_ca
                );
            }
        }
    }
}
