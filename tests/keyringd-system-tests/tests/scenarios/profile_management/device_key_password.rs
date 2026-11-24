#[cfg(target_os = "linux")]
use crate::scenario_steps;
#[cfg(target_os = "linux")]
use keyringd_system_tests::application::SystemTestApplication;
#[cfg(target_os = "linux")]
use keyringd_system_tests::application::features::profile_setup::CompletionPageDriver;
#[cfg(target_os = "linux")]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
#[cfg(target_os = "linux")]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
#[cfg(target_os = "linux")]
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
#[cfg(target_os = "linux")]
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_testkit::scenario;
#[cfg(target_os = "linux")]
const ORIGINAL_PASSWORD: &str = "correct horse battery staple";
#[cfg(target_os = "linux")]
const CHANGED_PASSWORD: &str = "troubadour battery staple correct";
#[cfg(target_os = "linux")]
const PROFILE_NAME: &str = "device-key-password-profile";
scenario! {
    group "Device Key / Password Settings" {
        #[cfg(target_os = "linux")]
        case "Enables, changes, persists, and disables the Device Key password" {
            given "Password Prepare a Profile with an unset Device Key" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let key = environment
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                key.authenticator().reset(keyringd_test_usbip::authenticator_server::client::AuthenticatorProfile::UvConfiguredPinUnconfigured).unwrap();
                key.authenticator().set_approval_mode(
                    keyringd_system_tests::authenticator::ApprovalMode::Manual,
                )
                .unwrap();
                environment.application().start(Default::default()).unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                scenario_steps::profile_setup::security_key_profiles::create_with_built_in_uv(
                    &mut environment,
                    &key,
                    PROFILE_NAME,
                );
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
            when "Open the Profile menu" {
                let profiles = environment.application().profiles_window().unwrap();
                let mut enable_menu = profiles.open_profile_actions_menu(PROFILE_NAME).unwrap();
            }
            then "Displays the items that enable Device Key Password" {
                scenario_steps::profile_management::linux_device_key_password::assert_menu_actions(
                    &enable_menu,
                    false,
                )
                .unwrap();
                enable_menu
                    .capture_screenshot("Enable local authentication menu action")
                    .unwrap();
                enable_menu.close().unwrap();
            }
            when "Enable and restart Device Key Password" {
                scenario_steps::profile_management::linux_device_key_password::enable(
                    &mut environment,
                    PROFILE_NAME,
                    ORIGINAL_PASSWORD,
                )
                .unwrap();
                environment.application().restart().unwrap();
            }
            then "Even after restarting, invalidation items are displayed and Password protection is preserved" {
                let profiles = environment.application().profiles_window().unwrap();
                let mut menu = profiles.open_profile_actions_menu(PROFILE_NAME).unwrap();
                scenario_steps::profile_management::linux_device_key_password::assert_menu_actions(
                    &menu, true,
                )
                .unwrap();
                menu.capture_screenshot("Disable local authentication menu action")
                    .unwrap();
                menu.close().unwrap();
            }
            when "Change Device Key Password and restart using your current Password" {
                scenario_steps::profile_management::linux_device_key_password::change(
                    &mut environment,
                    PROFILE_NAME,
                    ORIGINAL_PASSWORD,
                    CHANGED_PASSWORD,
                )
                .unwrap();
                environment.application().restart().unwrap();
            }
            then "Password protection still retained after restart" {
                let profiles = environment.application().profiles_window().unwrap();
                let mut menu = profiles.open_profile_actions_menu(PROFILE_NAME).unwrap();
                scenario_steps::profile_management::linux_device_key_password::assert_menu_actions(
                    &menu, true,
                )
                .unwrap();
                menu.capture_screenshot("Changed Device Key Password menu action")
                    .unwrap();
                menu.close().unwrap();
            }
            when "Disable and restart Device Key Password in the modified Password" {
                scenario_steps::profile_management::linux_device_key_password::disable(
                    &mut environment,
                    PROFILE_NAME,
                    CHANGED_PASSWORD,
                )
                .unwrap();
                environment.application().restart().unwrap();
            }
            then "Even after restarting, the valid items are displayed and the Password protection will be removed." {
                let profiles = environment.application().profiles_window().unwrap();
                let mut menu = profiles.open_profile_actions_menu(PROFILE_NAME).unwrap();
                scenario_steps::profile_management::linux_device_key_password::assert_menu_actions(
                    &menu, false,
                )
                .unwrap();
                menu.capture_screenshot("Device Key Password disabled")
                    .unwrap();
                menu.close().unwrap();
            }
        }
    }
}
