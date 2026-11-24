use keyringd_system_tests::application::features::profile_setup::CompletionPageDriver;
use keyringd_system_tests::application::features::profile_setup::PasskeySelectionPageDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;

use crate::scenario_steps;
use keyringd_system_tests::application::{ApplicationConfiguration, SystemTestApplication};
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_testkit::agent::AgentIdentity;
use keyringd_testkit::{evidence_text, scenario};

scenario! {
    group "Profile Recovery / Multiple Credentials" {
        case "Recovers using the selected Passkey when a USB key offers multiple credentials" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available() => "requires credential recovery fixture";
            given "2 Passkeys with different creation times and public keys" {
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
                let sources =
                    scenario_steps::profile_setup::RecoverySources::create(&mut environment, &key);
            }
            when "Select from the second and first order of display candidates and restore them." {
                for (index, name) in [(1, "chosen-second-passkey"), (0, "chosen-first-passkey")] {
                    environment
                        .application()
                        .profile_setup_window()
                        .open()
                        .unwrap();
                    let actual =
                        scenario_steps::profile_setup::recovery_selection::discover_credentials(
                            &mut environment,
                            &key,
                            name,
                        )
                        .unwrap();
                    let labels = sources.match_candidates(&actual);
                    evidence_text!("Displayed recovery candidates", actual.join("\n"));
                    environment
                        .application()
                        .capture_screenshot("Choose Passkey")
                        .unwrap();
                    environment
                        .application()
                        .profile_setup_window()
                        .wizard()
                        .unwrap()
                        .passkey_selection_page()
                        .unwrap()
                        .select_passkey(&labels[index])
                        .unwrap();
                    environment
                        .application()
                        .profile_setup_window()
                        .wizard()
                        .unwrap()
                        .passkey_selection_page()
                        .unwrap()
                        .continue_recovery()
                        .unwrap();
                    scenario_steps::profile_setup::recovery_selection::approve_selected(
                        &mut environment,
                        &key,
                    )
                    .unwrap();
                    scenario_steps::profile_setup::completion::wait_for(&mut environment).unwrap();
                    environment
                        .application()
                        .capture_screenshot("Recovered selected Passkey")
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
                    scenario_steps::profile_management::profile_list::assert_visible(
                        &environment.application().profiles_window().unwrap(),
                        name,
                    )
                    .unwrap();
                    let stored = environment.application().stored_profiles().unwrap();
                    assert_eq!(stored.len(), if index == 1 { 3 } else { 4 });
                    assert_eq!(stored.last().unwrap()["passkey"], sources.passkeys[index]);
                    let trust = scenario_steps::profile_management::public_keys::read(
                        &mut environment,
                        name,
                    )
                    .unwrap();
                    assert_eq!(trust.ca, sources.trust[index].ca);
                    assert_eq!(
                        AgentIdentity::from_openssh(&trust.passkey)
                            .unwrap()
                            .public_key,
                        AgentIdentity::from_openssh(&sources.trust[index].passkey)
                            .unwrap()
                            .public_key
                    );
                    evidence_text!("Recovered Certificate Authority Public Key", trust.ca);
                }
            }
            then "Only selected Passkey Profiles are added without changing the original 2" {
                let stored = environment.application().stored_profiles().unwrap();
                assert_eq!(stored[0]["passkey"], sources.passkeys[0]);
                assert_eq!(stored[1]["passkey"], sources.passkeys[1]);
            }
        }

        case "Resumes Security Key recovery after canceling Passkey selection" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available() => "requires credential recovery fixture";
            given "Service with 2 Passkey and wizard with open selection page" {
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
                let sources =
                    scenario_steps::profile_setup::RecoverySources::create(&mut environment, &key);
                let original = environment.application().stored_profiles().unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                let candidates =
                    scenario_steps::profile_setup::recovery_selection::discover_credentials(
                        &mut environment,
                        &key,
                        "cancel-selection",
                    )
                    .unwrap();
                sources.match_candidates(&candidates);
                environment
                    .application()
                    .capture_screenshot("Passkey selection before cancellation")
                    .unwrap();
            }
            when "Cancel on the selection page" {
                scenario_steps::profile_setup::navigation::cancel(&mut environment).unwrap();
                environment
                    .application()
                    .capture_screenshot("Profiles after setup cancellation")
                    .unwrap();
            }
            then "Close the wizard and only the original 2 are saved" {
                assert_eq!(
                    environment.application().stored_profiles().unwrap(),
                    original
                );
                scenario_steps::profile_management::profile_list::assert_absent(
                    &environment.application().profiles_window().unwrap(),
                    "cancel-selection",
                )
                .unwrap();
            }
            when "Reopen the wizard and recover from the same candidate" {
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                let actual =
                    scenario_steps::profile_setup::recovery_selection::discover_credentials(
                        &mut environment,
                        &key,
                        "after-selection-cancel",
                    )
                    .unwrap();
                let labels = sources.match_candidates(&actual);
                environment
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .passkey_selection_page()
                    .unwrap()
                    .select_passkey(&labels[1])
                    .unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .passkey_selection_page()
                    .unwrap()
                    .continue_recovery()
                    .unwrap();
                scenario_steps::profile_setup::recovery_selection::approve_selected(
                    &mut environment,
                    &key,
                )
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
            then "Add only one Passkey profile that you choose without leaving an old selection response" {
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "after-selection-cancel",
                )
                .unwrap();
                let stored = environment.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 3);
                assert_eq!(&stored[..2], original.as_slice());
                assert_eq!(stored[2]["passkey"], sources.passkeys[1]);
                let ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut environment,
                    "after-selection-cancel",
                )
                .unwrap();
                assert_eq!(ca, sources.trust[1].ca);
            }
        }
    }
}
