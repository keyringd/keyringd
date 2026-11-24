#[cfg(any(target_os = "macos", windows))]
use crate::scenario_steps;
#[cfg(target_os = "macos")]
use keyringd_system_tests::application::features::qr_prompt_window::QrPromptWindowDriver;
use keyringd_testkit::scenario;

#[cfg(target_os = "macos")]
use keyringd_system_tests::{
    application::{
        SystemTestApplication,
        features::profiles::{ProfileDetails, ProfileDetailsWindowDriver, ProfilesWindowDriver},
    },
    environment::{NativeSystemTestEnvironment, SystemTestEnvironment},
    ssh::{SshServers, SshSession, SshTestServer},
};
#[cfg(target_os = "macos")]
use keyringd_testkit::agent::AgentIdentity;
#[cfg(target_os = "macos")]
const PROFILE_NAME: &str = "device-key-auth";
scenario! {
    group "Device Key / Local Authentication Settings" {
        #[cfg(target_os = "macos")]
        case "Persists Device Key authentication settings and can still authenticate over SSH after changes" {
            given "Prepare an Authorization Unset Profile" {
                let (mut environment, _key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&[PROFILE_NAME])
                    .unwrap();
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    PROFILE_NAME,
                )
                .unwrap();
            }
            when "Activate and restart authorization from the Profile menu" {
                {
                    let profiles = environment.application().profiles_window().unwrap();
                    let protection = profiles
                        .open_device_key_protection(PROFILE_NAME, true)
                        .unwrap();
                    protection
                        .capture_screenshot("enable-local-authentication-confirmation")
                        .unwrap();
                    protection.confirm().unwrap();
                    protection
                        .qr_prompt()
                        .unwrap()
                        .press_use_security_key()
                        .unwrap();
                    protection
                        .trust_entries_window(PROFILE_NAME)
                        .unwrap()
                        .close()
                        .unwrap();
                }
                scenario_steps::profile_management::profile_repository::wait_for(
                    &mut environment,
                    |profiles| {
                        profiles.iter().any(|profile| {
                            profile["displayName"] == PROFILE_NAME
                                && profile["deviceKey"]["localAuthentication"] == "required"
                        })
                    },
                )
                .unwrap();
                environment.application().restart().unwrap();
            }
            then "Authorization Required will appear in Profile Details after restarting" {
                {
                    let profiles = environment.application().profiles_window().unwrap();
                    let details = profiles.open_details(PROFILE_NAME).unwrap();
                    assert_eq!(
                        details.content().unwrap(),
                        ProfileDetails {
                            status: "Enabled".into(),
                            signature_algorithm: "ecdsa-sha2-nistp256".into(),
                            protection: "Local authentication required".into()
                        }
                    );
                    details.capture_screenshot("authorization-enabled").unwrap();
                    details.close().unwrap();
                }
                let device_key = scenario_steps::profile_management::public_keys::copy_device_key(
                    &mut environment,
                    PROFILE_NAME,
                )
                .unwrap();
                let identity = AgentIdentity::from_openssh(&device_key).unwrap();
                assert_eq!(
                    identity.comment,
                    format!(
                        "keyringd {PROFILE_NAME} device key@{}",
                        gethostname::gethostname().to_string_lossy()
                    )
                );
                let server = environment.ssh_servers().start(&device_key).unwrap();
                let output = server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        &identity,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap()
                    .finish()
                    .unwrap();
                output.assert_authenticated(&identity);
                case_execution.attach_log(
                    "sshd after authorization setting restart",
                    &output.server_log,
                );
                case_execution.attach_log("SSH after authorization setting restart", output.stderr);
            }
            when "Disable authorization from the Profile menu and restart" {
                {
                    let profiles = environment.application().profiles_window().unwrap();
                    let protection = profiles
                        .open_device_key_protection(PROFILE_NAME, false)
                        .unwrap();
                    protection
                        .capture_screenshot("disable-local-authentication-confirmation")
                        .unwrap();
                    protection.confirm().unwrap();
                    protection
                        .qr_prompt()
                        .unwrap()
                        .press_use_security_key()
                        .unwrap();
                    protection
                        .trust_entries_window(PROFILE_NAME)
                        .unwrap()
                        .close()
                        .unwrap();
                }
                scenario_steps::profile_management::profile_repository::wait_for(
                    &mut environment,
                    |profiles| {
                        profiles.iter().any(|profile| {
                            profile["displayName"] == PROFILE_NAME
                                && profile["deviceKey"]["localAuthentication"] == "notRequired"
                        })
                    },
                )
                .unwrap();
                environment.application().restart().unwrap();
            }
            then "Authorization has been canceled even after restarting" {
                {
                    let profiles = environment.application().profiles_window().unwrap();
                    let details = profiles.open_details(PROFILE_NAME).unwrap();
                    assert_eq!(
                        details.content().unwrap(),
                        ProfileDetails {
                            status: "Enabled".into(),
                            signature_algorithm: "ecdsa-sha2-nistp256".into(),
                            protection: "No local authentication".into()
                        }
                    );
                    details
                        .capture_screenshot("authorization-disabled")
                        .unwrap();
                    details.close().unwrap();
                }
                let device_key = scenario_steps::profile_management::public_keys::copy_device_key(
                    &mut environment,
                    PROFILE_NAME,
                )
                .unwrap();
                let identity = AgentIdentity::from_openssh(&device_key).unwrap();
                assert_eq!(
                    identity.comment,
                    format!(
                        "keyringd {PROFILE_NAME} device key@{}",
                        gethostname::gethostname().to_string_lossy()
                    )
                );
                let server = environment.ssh_servers().start(&device_key).unwrap();
                let output = server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        &identity,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap()
                    .finish()
                    .unwrap();
                output.assert_authenticated(&identity);
                case_execution.attach_log(
                    "sshd after authorization setting restart",
                    &output.server_log,
                );
                case_execution.attach_log("SSH after authorization setting restart", output.stderr);
            }
        }
    }
}

#[cfg(target_os = "windows")]
use keyringd_core::service::DeviceKeyProtectionChange;
#[cfg(target_os = "windows")]
use keyringd_system_tests::{
    application::{
        ApplicationConfiguration, SystemTestApplication,
        features::{
            profile_setup::{
                CompletionPageDriver, ProfileSetupWindowDriver, ProfileSetupWizardDriver,
            },
            profiles::{ProfileDetailsWindowDriver, ProfilesWindowDriver},
        },
    },
    environment::{NativeSystemTestEnvironment, SystemTestEnvironment},
};
scenario! {
    group "Device Key / Local Authentication Settings" {
        #[cfg(target_os = "windows")]
        case "Enables, reconfigures, persists, and disables Device Key authentication" {
            // Software Device Key exercises product confirmation, replacement and persistence; native TPM/Windows Security authorization has its own case.
            given "Verification-free Profile and original Trust Entry" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                environment
                    .application()
                    .start(ApplicationConfiguration {
                        test_fido_mode: Some(keyringd_core::test_support::TestFidoMode::Success),
                        ..Default::default()
                    })
                    .unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                scenario_steps::profile_setup::form::submit_creation(
                    &mut environment,
                    "verification",
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
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "verification",
                )
                .unwrap();
                let original = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    "verification",
                )
                .unwrap();
                let original_ca = original.ca;
                let original_key = original.device;
                let requests = environment.application().test_fido_request_count().unwrap();
            }
            when "Cancel validation confirmation once" {
                let profiles = environment.application().profiles_window().unwrap();
                let confirmation = profiles
                    .open_device_key_protection("verification", DeviceKeyProtectionChange::Enable)
                    .unwrap();
                confirmation
                    .capture_screenshot("Enable verification confirmation")
                    .unwrap();
                confirmation.cancel().unwrap();
            }
            then "Maintains original key and verification unnecessary" {
                assert_eq!(
                    environment.application().test_fido_request_count().unwrap(),
                    requests
                );
                assert_eq!(
                    scenario_steps::profile_management::public_keys::copy_device_key(
                        &mut environment,
                        "verification"
                    )
                    .unwrap(),
                    original_key
                );
                let profiles = environment.application().profiles_window().unwrap();
                let details = profiles.open_details("verification").unwrap();
                assert_eq!(
                    details.content().unwrap().protection,
                    "No local authentication"
                );
                details.close().unwrap();
            }
            when "Enable Verification" {
                let previous = environment.application().stored_profiles().unwrap();
                {
                    let profiles = environment.application().profiles_window().unwrap();
                    let confirmation = profiles
                        .open_device_key_protection(
                            "verification",
                            DeviceKeyProtectionChange::Enable,
                        )
                        .unwrap();
                    confirmation.continue_change().unwrap();
                }
                scenario_steps::profile_management::profile_repository::wait_for(
                    &mut environment,
                    |profiles| profiles != previous,
                )
                .unwrap();
                environment
                    .application()
                    .trust_entries_window("verification")
                    .unwrap()
                    .close()
                    .unwrap();
                let enabled_key = scenario_steps::profile_management::public_keys::copy_device_key(
                    &mut environment,
                    "verification",
                )
                .unwrap();
            }
            then "Maintaining CA and swapped for independent keys and verification required" {
                assert_ne!(enabled_key, original_key);
                assert!(enabled_key.starts_with("ecdsa-sha2-nistp256 "));
                assert_eq!(
                    scenario_steps::profile_management::public_keys::copy_ca(
                        &mut environment,
                        "verification"
                    )
                    .unwrap(),
                    original_ca
                );
                {
                    let profiles = environment.application().profiles_window().unwrap();
                    let details = profiles.open_details("verification").unwrap();
                    assert_eq!(
                        details.content().unwrap().protection,
                        "Local authentication required"
                    );
                    details
                        .capture_screenshot("Authentication required")
                        .unwrap();
                    details.close().unwrap();
                }
                assert_eq!(
                    environment.application().test_fido_request_count().unwrap(),
                    requests + 1
                );
            }
            when "Get public keys after operation" {
                let previous = environment.application().stored_profiles().unwrap();
                {
                    let profiles = environment.application().profiles_window().unwrap();
                    let confirmation = profiles
                        .open_device_key_protection(
                            "verification",
                            DeviceKeyProtectionChange::ChangePassword,
                        )
                        .unwrap();
                    confirmation
                        .capture_screenshot("Reconfigure verification (Software Device Key)")
                        .unwrap();
                    confirmation.continue_change().unwrap();
                }
                scenario_steps::profile_management::profile_repository::wait_for(
                    &mut environment,
                    |profiles| profiles != previous,
                )
                .unwrap();
                environment
                    .application()
                    .trust_entries_window("verification")
                    .unwrap()
                    .close()
                    .unwrap();
                let reconfigured_key =
                    scenario_steps::profile_management::public_keys::copy_device_key(
                        &mut environment,
                        "verification",
                    )
                    .unwrap();
            }
            then "Update and reauthorize Device Key with Verification Reconfiguration" {
                assert_ne!(reconfigured_key, enabled_key);
                assert_eq!(
                    environment.application().test_fido_request_count().unwrap(),
                    requests + 2
                );
            }
            when "Restart the app with the same save destination" {
                environment.application().restart().unwrap();
            }
            then "Verification Required/replaced key/CA persists" {
                {
                    let profiles = environment.application().profiles_window().unwrap();
                    let details = profiles.open_details("verification").unwrap();
                    assert_eq!(
                        details.content().unwrap().protection,
                        "Local authentication required"
                    );
                    details
                        .capture_screenshot("Verification required after restart")
                        .unwrap();
                    details.close().unwrap();
                }
                let entries = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    "verification",
                )
                .unwrap();
                assert_eq!(entries.device, reconfigured_key);
                assert_eq!(entries.ca, original_ca);
            }
            when "Get public keys after operation" {
                let previous = environment.application().stored_profiles().unwrap();
                {
                    let profiles = environment.application().profiles_window().unwrap();
                    let confirmation = profiles
                        .open_device_key_protection(
                            "verification",
                            DeviceKeyProtectionChange::Disable,
                        )
                        .unwrap();
                    confirmation
                        .capture_screenshot("Disable verification (Software Device Key)")
                        .unwrap();
                    confirmation.continue_change().unwrap();
                }
                scenario_steps::profile_management::profile_repository::wait_for(
                    &mut environment,
                    |profiles| profiles != previous,
                )
                .unwrap();
                environment
                    .application()
                    .trust_entries_window("verification")
                    .unwrap()
                    .close()
                    .unwrap();
                let disabled_key =
                    scenario_steps::profile_management::public_keys::copy_device_key(
                        &mut environment,
                        "verification",
                    )
                    .unwrap();
            }
            then "Update to non-SK Device Key with Verification Disable" {
                assert_ne!(disabled_key, reconfigured_key);
                assert!(disabled_key.starts_with("ecdsa-sha2-nistp256 "));
            }
            when "Restart the app with the same save destination" {
                environment.application().restart().unwrap();
            }
            then "Verification unneeded state and new key persisted and CA is maintained" {
                {
                    let profiles = environment.application().profiles_window().unwrap();
                    let details = profiles.open_details("verification").unwrap();
                    assert_eq!(
                        details.content().unwrap().protection,
                        "No local authentication"
                    );
                    details
                        .capture_screenshot("Verification disabled after restart")
                        .unwrap();
                    details.close().unwrap();
                }
                let entries = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    "verification",
                )
                .unwrap();
                assert_eq!(entries.device, disabled_key);
                assert_eq!(entries.ca, original_ca);
            }
        }
    }
}
