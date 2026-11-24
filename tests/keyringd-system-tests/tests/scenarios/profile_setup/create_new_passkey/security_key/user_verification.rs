#[cfg(unix)]
use crate::scenario_steps;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::CompletionPageDriver;
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
#[cfg(any(target_os = "linux", target_os = "macos"))]
use keyringd_test_usbip::authenticator_server::client::AuthenticatorProfile;

use keyringd_testkit::scenario;

scenario! {
    group "Profile Setup / Security Key User Verification" {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        case "Prefers built-in UV when UV and PIN are both configured" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#gettingPinUvAuthToken"
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and the OS Security Key environment";
            given "UV-configured Security Key and Empty Applications" {
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
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
            }
            when "Approve Passkey creation and Device Key authorization with built-in UV" {
                scenario_steps::profile_setup::security_key_profiles::create_with_built_in_uv(
                    &mut driver,
                    &key,
                    "uv-pin-profile",
                );
            }
            then "You can complete a Profile without requiring a PIN" {
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
                    "uv-pin-profile",
                )
                .unwrap();
                let stored = driver.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "uv-pin-profile");
            }
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        case "Uses configured built-in UV without creating a PIN" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#gettingPinUvAuthToken"
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and the OS Security Key environment";
            given "UV-configured Security Key and Empty Applications" {
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
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
            }
            when "Approve Passkey creation and Device Key authorization with built-in UV" {
                scenario_steps::profile_setup::security_key_profiles::create_with_built_in_uv(
                    &mut driver,
                    &key,
                    "uv-only-profile",
                );
            }
            then "You can complete a Profile without requiring a PIN" {
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
                    "uv-only-profile",
                )
                .unwrap();
                let stored = driver.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "uv-only-profile");
            }
        }
    }
}

scenario! {
    group "Profile Setup / Security Key PIN" {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        case "Uses the configured PIN to get a token when built-in UV is unavailable" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#gettingPinUvAuthToken"
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and direct CTAP Security Key support";
            given "The Security Key with an unset internal UV and an empty location" {
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
            }
            when "Enter or configure a PIN and approve each RP operation" {
                scenario_steps::profile_setup::security_key_profiles::create_with_pin(
                    &mut driver,
                    &key,
                    "configured-pin-profile",
                    false,
                );
            }
            then "Create a Profile with a PIN in two steps" {
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
                    "configured-pin-profile",
                )
                .unwrap();
                let stored = driver.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "configured-pin-profile");
            }
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        case "Sets a PIN and gets a token when neither PIN nor UV is configured" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#settingNewPin"
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and direct CTAP Security Key support";
            given "The Security Key with an unset internal UV and an empty location" {
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
            }
            when "Enter or configure a PIN and approve each RP operation" {
                scenario_steps::profile_setup::security_key_profiles::create_with_pin(
                    &mut driver,
                    &key,
                    "new-pin-profile",
                    true,
                );
            }
            then "Create a Profile with a PIN in two steps" {
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
                    "new-pin-profile",
                )
                .unwrap();
                let stored = driver.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "new-pin-profile");
            }
        }
    }
}
