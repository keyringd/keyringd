use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, PasskeyNotFoundPageDriver,
};

use crate::scenario_steps;
use keyringd_system_tests::application::{ApplicationConfiguration, SystemTestApplication};
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_testkit::agent::AgentIdentity;
use keyringd_testkit::{evidence_text, scenario};

scenario! {
    group "Profile Recovery / Security Key" {
        case "Retries recovery with another device when the Security Key has no matching Passkey" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available() => "requires credential recovery fixture";
            given "Prepare a key with a passkey and an empty key with the saved profile left" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let key = environment
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                let configuration = ApplicationConfiguration::default();
                // Windows WebAuthn can also see physical Security Keys attached to the
                // developer's machine; disconnecting our virtual keys does not hide them.
                // Use TestPasskeyService so NoCredentials and retry are deterministic,
                // independent of those devices. This case checks the UI after that error.
                #[cfg(windows)]
                let configuration = ApplicationConfiguration {
                    test_fido_mode: Some(keyringd_core::test_support::TestFidoMode::Success),
                    ..configuration
                };
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
                let original_profiles = environment.application().stored_profiles().unwrap();
                let mut expected_passkey = original_profiles[0]["passkey"].clone();
                if cfg!(windows) {
                    // The Windows test service has no largeBlob storage for registration evidence.
                    expected_passkey["attestationResponse"] = serde_json::Value::Null;
                }
                let original = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    "source-passkey",
                )
                .unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                #[cfg(unix)]
                {
                    key.disconnect().unwrap();
                    other_key.authenticator()
                        .set_approval_mode(
                            keyringd_system_tests::authenticator::ApprovalMode::Manual,
                        )
                        .unwrap();
                }
                #[cfg(windows)]
                environment
                    .application()
                    .set_test_fido_mode(keyringd_core::test_support::TestFidoMode::NoCredentials)
                    .unwrap();
                assert_eq!(
                    environment.application().stored_profiles().unwrap(),
                    original_profiles
                );
            }
            when "Try recovery with an empty key" {
                scenario_steps::profile_setup::recovery_selection::discover_missing_passkey(
                    &mut environment,
                    &other_key,
                    "missing-passkey",
                )
                .unwrap();
            }
            then "Show Passkey Absence and Don't Save Profile" {
                environment
                    .application()
                    .capture_screenshot("No Passkey on Security Key")
                    .unwrap();
                assert_eq!(
                    environment.application().stored_profiles().unwrap(),
                    original_profiles
                );
            }
            when "Back to the same input and search again with the same wizard" {
                environment
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .back()
                    .unwrap();
                scenario_steps::profile_setup::form::assert_content(
                    &mut environment,
                    "missing-passkey",
                    true,
                )
                .unwrap();
                scenario_steps::profile_setup::recovery_selection::discover_missing_passkey(
                    &mut environment,
                    &other_key,
                    "missing-passkey",
                )
                .unwrap();
            }
            then "It is not saved even if you see the absence again" {
                assert_eq!(
                    environment.application().stored_profiles().unwrap(),
                    original_profiles
                );
            }
            when "Switch to another key with the original Passkey and retry from the same wizard" {
                #[cfg(unix)]
                {
                    other_key.disconnect().unwrap();
                    key.connect().unwrap();
                }
                #[cfg(windows)]
                environment
                    .application()
                    .set_test_fido_mode(keyringd_core::test_support::TestFidoMode::Success)
                    .unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .passkey_not_found_page()
                    .unwrap()
                    .try_another_device()
                    .unwrap();
                scenario_steps::profile_setup::recovery_selection::resume(&mut environment)
                    .unwrap();
                scenario_steps::profile_setup::completion::wait_for(&mut environment).unwrap();
                environment
                    .application()
                    .capture_screenshot("Recovery after retry")
                    .unwrap();
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
            then "Add only one profile from the same Passkey/CA without changing the original profile" {
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "missing-passkey",
                )
                .unwrap();
                let stored = environment.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 2);
                assert_eq!(stored[0], original_profiles[0]);
                assert_eq!(stored[1]["passkey"], expected_passkey);
                let recovered = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    "missing-passkey",
                )
                .unwrap();
                assert_eq!(recovered.ca, original.ca);
                assert_eq!(
                    AgentIdentity::from_openssh(&recovered.passkey)
                        .unwrap()
                        .public_key,
                    AgentIdentity::from_openssh(&original.passkey)
                        .unwrap()
                        .public_key
                );
                evidence_text!("Restored CA", recovered.ca);
            }
        }
    }
}
