use crate::scenario_steps;
use keyringd_system_tests::application::SystemTestApplication;
#[cfg(unix)]
use keyringd_system_tests::application::features::profiles::ProfilesWindowDriver;
#[cfg(unix)]
use keyringd_system_tests::application::features::profiles::RemoveProfileWindowDriver;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_testkit::scenario;

scenario! {
    group "Profile Management" {
        case "Keeps a renamed profile’s display name" {
            given "Prepare a created profile" {
                let (mut environment, _key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&["before-rename"])
                    .unwrap();
            }
            when "Change profile name" {
                let profiles = environment.application().profiles_window().unwrap();
                profiles.begin_rename("before-rename").unwrap();
                profiles
                    .enter_profile_name("before-rename", "after-rename")
                    .unwrap();
                assert_eq!(
                    profiles
                        .profile_name_editor_content("before-rename")
                        .unwrap(),
                    "after-rename"
                );
                profiles.capture_screenshot("Profile name editor").unwrap();
                profiles.confirm_rename("before-rename").unwrap();
            }
            then "Only new names exist in the list and save content" {
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "after-rename",
                )
                .unwrap();
                scenario_steps::profile_management::profile_list::assert_absent(
                    &environment.application().profiles_window().unwrap(),
                    "before-rename",
                )
                .unwrap();
                let stored = scenario_steps::profile_management::profile_repository::wait_for(
                    &mut environment,
                    |p| p.len() == 1 && p[0]["displayName"] == "after-rename",
                )
                .unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "after-rename");
                environment
                    .application()
                    .capture_screenshot("Renamed profile")
                    .unwrap();
            }
            when "Restart the application with the same destination" {
                environment.application().restart().unwrap();
            }
            then "The changed name is preserved and the old name is not displayed" {
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "after-rename",
                )
                .unwrap();
                scenario_steps::profile_management::profile_list::assert_absent(
                    &environment.application().profiles_window().unwrap(),
                    "before-rename",
                )
                .unwrap();
                environment
                    .application()
                    .capture_screenshot("Renamed profile after restart")
                    .unwrap();
            }
        }

        case "Can re-enable a disabled profile after restarting" {
            given "Prepare a valid profile" {
                let (mut environment, _key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&["toggle-profile"])
                    .unwrap();
                scenario_steps::profile_management::profile_list::assert_enabled(
                    &environment.application().profiles_window().unwrap(),
                    "toggle-profile",
                    true,
                )
                .unwrap();
            }
            when "Disable Profiles from List" {
                environment
                    .application()
                    .profiles_window()
                    .unwrap()
                    .toggle_profile("toggle-profile")
                    .unwrap();
            }
            then "The invalid state is reflected on the screen and saved contents" {
                scenario_steps::profile_management::profile_list::assert_enabled(
                    &environment.application().profiles_window().unwrap(),
                    "toggle-profile",
                    false,
                )
                .unwrap();
                let stored = scenario_steps::profile_management::profile_repository::wait_for(
                    &mut environment,
                    |p| p.len() == 1 && p[0]["enabled"] == false,
                )
                .unwrap();
                assert_eq!(stored[0]["enabled"], false);
                environment
                    .application()
                    .capture_screenshot("Disabled profile")
                    .unwrap();
            }
            when "Restart the application with the same destination" {
                environment.application().restart().unwrap();
            }
            then "Disabled state is retained" {
                scenario_steps::profile_management::profile_list::assert_enabled(
                    &environment.application().profiles_window().unwrap(),
                    "toggle-profile",
                    false,
                )
                .unwrap();
            }
            when "Re-enable it from the list" {
                environment
                    .application()
                    .profiles_window()
                    .unwrap()
                    .toggle_profile("toggle-profile")
                    .unwrap();
            }
            then "The valid state is reflected in the screen and saved contents" {
                scenario_steps::profile_management::profile_list::assert_enabled(
                    &environment.application().profiles_window().unwrap(),
                    "toggle-profile",
                    true,
                )
                .unwrap();
                let stored = scenario_steps::profile_management::profile_repository::wait_for(
                    &mut environment,
                    |p| p.len() == 1 && p[0]["enabled"] == true,
                )
                .unwrap();
                assert_eq!(stored[0]["enabled"], true);
                environment
                    .application()
                    .capture_screenshot("Reenabled profile after restart")
                    .unwrap();
            }
        }

        case "Keeps a deleted profile removed after restarting" {
            given "Prepare profiles for deletion and retention targets" {
                let (mut environment, _key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&["remove-me", "keep-me"])
                    .unwrap();
            }
            when "Remove target from confirmation dialog" {
                let mut profiles = environment.application().profiles_window().unwrap();
                let mut removal = profiles.open_removal("remove-me").unwrap();
                removal
                    .capture_screenshot("Remove profile confirmation")
                    .unwrap();
                removal.confirm().unwrap();
            }
            then "Only the target disappears from the list and save contents" {
                scenario_steps::profile_management::profile_list::assert_absent(
                    &environment.application().profiles_window().unwrap(),
                    "remove-me",
                )
                .unwrap();
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "keep-me",
                )
                .unwrap();
                let stored = scenario_steps::profile_management::profile_repository::wait_for(
                    &mut environment,
                    |p| p.len() == 1 && p[0]["displayName"] == "keep-me",
                )
                .unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "keep-me");
                environment
                    .application()
                    .capture_screenshot("Profile removed")
                    .unwrap();
            }
            when "Restart the application with the same destination" {
                environment.application().restart().unwrap();
            }
            then "The deletion target is not restored and the retained target remains" {
                scenario_steps::profile_management::profile_list::assert_absent(
                    &environment.application().profiles_window().unwrap(),
                    "remove-me",
                )
                .unwrap();
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "keep-me",
                )
                .unwrap();
                environment
                    .application()
                    .capture_screenshot("Profiles after restart")
                    .unwrap();
            }
        }

        case "Keeps the drag-and-drop profile order after restarting" {
            skip_if: !<<NativeSystemTestEnvironment as SystemTestEnvironment>::Application as SystemTestApplication>::drag_supported().unwrap() => "Elevated WinUI cannot use standard drag and drop. The menu cases cover reordering and persistence.";
            given "Prepare 2 profiles" {
                let (mut environment, _key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&["first-profile", "second-profile"])
                    .unwrap();
                scenario_steps::profile_management::profile_list::assert_above(
                    &environment.application().profiles_window().unwrap(),
                    "first-profile",
                    "second-profile",
                )
                .unwrap();
                environment
                    .application()
                    .capture_screenshot("Profiles before reordering")
                    .unwrap();
            }
            when "Drag the 2nd case above the 1st case" {
                environment
                    .application()
                    .profiles_window()
                    .unwrap()
                    .reorder_before("second-profile", "first-profile")
                    .unwrap();
            }
            then "Display order and save order are switched" {
                scenario_steps::profile_management::profile_list::assert_above(
                    &environment.application().profiles_window().unwrap(),
                    "second-profile",
                    "first-profile",
                )
                .unwrap();
                let stored = scenario_steps::profile_management::profile_repository::wait_for(
                    &mut environment,
                    |p| {
                        p.len() == 2
                            && p[0]["displayName"] == "second-profile"
                            && p[1]["displayName"] == "first-profile"
                    },
                )
                .unwrap();
                assert_eq!(stored[0]["displayName"], "second-profile");
                assert_eq!(stored[1]["displayName"], "first-profile");
                environment
                    .application()
                    .capture_screenshot("Reordered profiles")
                    .unwrap();
            }
            when "Restart the application with the same destination" {
                environment.application().restart().unwrap();
            }
            then "Rearranged display order is preserved" {
                scenario_steps::profile_management::profile_list::assert_above(
                    &environment.application().profiles_window().unwrap(),
                    "second-profile",
                    "first-profile",
                )
                .unwrap();
                environment
                    .application()
                    .capture_screenshot("Profile order after restart")
                    .unwrap();
            }
        }
    }
}
