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
    group "Profile Recovery / Hybrid / Multiple Credentials" {
        case "Recovers using the selected Passkey when a phone offers multiple credentials" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::hybrid_available() => "Requires a virtual phone and a Hybrid test tunnel";
            given "2 Passkeys with different creation times and public keys" {
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
                let configuration = ApplicationConfiguration {
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
                let sources = scenario_steps::profile_setup::RecoverySources::create_hybrid(
                    &mut environment,
                    &phone,
                );
            }
            when "Select from the second and first order of display candidates and restore them." {
                for (index, name) in [(1, "chosen-second-passkey"), (0, "chosen-first-passkey")] {
                    environment
                        .application()
                        .profile_setup_window()
                        .open()
                        .unwrap();
                    let actual =
                        scenario_steps::profile_setup::hybrid_profiles::discover_credentials(
                            &mut environment,
                            &phone,
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
                    scenario_steps::profile_setup::hybrid_profiles::approve_selected(
                        &mut environment,
                        &phone,
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
                    let profiles = environment.application().profiles_window().unwrap();
                    #[cfg(windows)]
                    profiles.reveal_newest_profile(name).unwrap();
                    scenario_steps::profile_management::profile_list::assert_visible(&profiles, name)
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

        case "Resumes Hybrid recovery after canceling Passkey selection" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::hybrid_available() => "Requires a virtual phone and a Hybrid test tunnel";
            given "Phone with 2 Passkey and Wizard Open Selected Page" {
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
                let configuration = ApplicationConfiguration {
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
                let sources = scenario_steps::profile_setup::RecoverySources::create_hybrid(
                    &mut environment,
                    &phone,
                );
                let original = environment.application().stored_profiles().unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                let candidates =
                    scenario_steps::profile_setup::hybrid_profiles::discover_credentials(
                        &mut environment,
                        &phone,
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
                let actual = scenario_steps::profile_setup::hybrid_profiles::discover_credentials(
                    &mut environment,
                    &phone,
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
                scenario_steps::profile_setup::hybrid_profiles::approve_selected(
                    &mut environment,
                    &phone,
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
