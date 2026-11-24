use crate::scenario_steps;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_testkit::scenario;

use keyringd_system_tests::application::features::profiles::ProfilesWindowDriver;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;

scenario! {
    group "Profile Management" {
        case "Keeps profile order after rearranging with the menu and restarting" {
            given "3 Profiles Created from UI" {
                let (mut environment, _key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&["first", "second", "third"])
                    .unwrap();
            }
            then "The head cannot move up and the end cannot move downward" {
                assert_eq!(
                    environment
                        .application()
                        .profiles_window()
                        .unwrap()
                        .profile_move_actions_enabled("first")
                        .unwrap(),
                    (false, true)
                );
                assert_eq!(
                    environment
                        .application()
                        .profiles_window()
                        .unwrap()
                        .profile_move_actions_enabled("second")
                        .unwrap(),
                    (true, true)
                );
                assert_eq!(
                    environment
                        .application()
                        .profiles_window()
                        .unwrap()
                        .profile_move_actions_enabled("third")
                        .unwrap(),
                    (true, false)
                );
            }
            when "Move the central profile to the top" {
                environment
                    .application()
                    .profiles_window()
                    .unwrap()
                    .move_profile_up("second")
                    .unwrap();
            }
            then "Profiles line up in the specified order" {
                scenario_steps::profile_management::profile_list::assert_above(
                    &environment.application().profiles_window().unwrap(),
                    "second",
                    "first",
                )
                .unwrap();
            }
            then "The save order and destination menu status are updated." {
                let stored = scenario_steps::profile_management::profile_repository::wait_for(
                    &mut environment,
                    |profiles| {
                        profiles
                            .iter()
                            .map(|profile| profile["displayName"].as_str())
                            .collect::<Vec<_>>()
                            == [Some("second"), Some("first"), Some("third")]
                    },
                )
                .unwrap();
                assert_eq!(
                    stored
                        .iter()
                        .map(|profile| profile["displayName"].as_str())
                        .collect::<Vec<_>>(),
                    [Some("second"), Some("first"), Some("third")]
                );
                assert_eq!(
                    environment
                        .application()
                        .profiles_window()
                        .unwrap()
                        .profile_move_actions_enabled("second")
                        .unwrap(),
                    (false, true)
                );
                assert_eq!(
                    environment
                        .application()
                        .profiles_window()
                        .unwrap()
                        .profile_move_actions_enabled("first")
                        .unwrap(),
                    (true, true)
                );
            }
            when "Continue to move to the end of the same profile" {
                environment
                    .application()
                    .profiles_window()
                    .unwrap()
                    .move_profile_down("second")
                    .unwrap();
            }
            then "Profiles line up in the specified order" {
                scenario_steps::profile_management::profile_list::assert_above(
                    &environment.application().profiles_window().unwrap(),
                    "first",
                    "second",
                )
                .unwrap();
            }
            when "Continue to move to the end of the same profile" {
                environment
                    .application()
                    .profiles_window()
                    .unwrap()
                    .move_profile_down("second")
                    .unwrap();
            }
            then "Profiles line up in the specified order" {
                scenario_steps::profile_management::profile_list::assert_above(
                    &environment.application().profiles_window().unwrap(),
                    "third",
                    "second",
                )
                .unwrap();
            }
            then "The save order and the menu state at the end are updated" {
                let stored = scenario_steps::profile_management::profile_repository::wait_for(
                    &mut environment,
                    |profiles| {
                        profiles
                            .iter()
                            .map(|profile| profile["displayName"].as_str())
                            .collect::<Vec<_>>()
                            == [Some("first"), Some("third"), Some("second")]
                    },
                )
                .unwrap();
                assert_eq!(
                    stored
                        .iter()
                        .map(|profile| profile["displayName"].as_str())
                        .collect::<Vec<_>>(),
                    [Some("first"), Some("third"), Some("second")]
                );
                assert_eq!(
                    environment
                        .application()
                        .profiles_window()
                        .unwrap()
                        .profile_move_actions_enabled("second")
                        .unwrap(),
                    (true, false)
                );
                assert_eq!(
                    environment
                        .application()
                        .profiles_window()
                        .unwrap()
                        .profile_move_actions_enabled("third")
                        .unwrap(),
                    (true, true)
                );
                environment
                    .application()
                    .capture_screenshot("Profiles reordered from the menu")
                    .unwrap();
            }
            when "Restart the app with the same save destination" {
                environment.application().restart().unwrap();
            }
            then "The display order and boundary invalid states are preserved after moving." {
                let stored = environment.application().stored_profiles().unwrap();
                assert_eq!(
                    stored
                        .iter()
                        .map(|profile| profile["displayName"].as_str())
                        .collect::<Vec<_>>(),
                    [Some("first"), Some("third"), Some("second")]
                );
                scenario_steps::profile_management::profile_list::assert_above(
                    &environment.application().profiles_window().unwrap(),
                    "first",
                    "third",
                )
                .unwrap();
                scenario_steps::profile_management::profile_list::assert_above(
                    &environment.application().profiles_window().unwrap(),
                    "third",
                    "second",
                )
                .unwrap();
                assert_eq!(
                    environment
                        .application()
                        .profiles_window()
                        .unwrap()
                        .profile_move_actions_enabled("first")
                        .unwrap(),
                    (false, true)
                );
                assert_eq!(
                    environment
                        .application()
                        .profiles_window()
                        .unwrap()
                        .profile_move_actions_enabled("second")
                        .unwrap(),
                    (true, false)
                );
                environment
                    .application()
                    .capture_screenshot("Menu order after restart")
                    .unwrap();
            }
        }
    }
}
