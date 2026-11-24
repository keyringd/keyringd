use crate::scenario_steps;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profiles::ProfileDetailsWindowDriver;
use keyringd_system_tests::application::features::profiles::TrustEntriesWindowDriver;
use keyringd_system_tests::application::features::profiles::{
    ProfileDetails, ProfilesWindowDriver,
};
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_testkit::{evidence_text, scenario};

scenario! {
    group "Profile Management / Details and Public Keys" {
        case "Shows profile details and copies each trust entry from the management menu" {
            given "Prepare a created Profile" {
                let (mut environment, _key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&["profile-details"])
                    .unwrap();
            }
            when "Open Details, read and close the displayed values" {
                let mut profiles = environment.application().profiles_window().unwrap();
                let mut window = profiles.open_details("profile-details").unwrap();
                let details = window.content().unwrap();
                window.capture_screenshot("Profile Details").unwrap();
                window.close().unwrap();
            }
            then "Status, signature format, and protection status are displayed correctly" {
                assert_eq!(
                    details,
                    ProfileDetails {
                        status: "Enabled".into(),
                        signature_algorithm: "ecdsa-sha2-nistp256".into(),
                        protection: "No local authentication".into(),
                    }
                );
            }
            when "Open Public Keys from the admin menu and run each entry and bulk copy" {
                let mut profiles = environment.application().profiles_window().unwrap();
                let mut window = profiles.open_trust_entries("profile-details").unwrap();
                let displayed = window.displayed_entries().unwrap();
                let ca = window.copy_ca_trust_entry().unwrap();
                let device = window.copy_device_key_trust_entry().unwrap();
                let passkey = window.copy_passkey_trust_entry().unwrap();
                let combined = window.copy_direct_key_trust_entries().unwrap();
            }
            then "A copy of what is actually copied is also displayed correctly" {
                assert_eq!(displayed, [&ca, &device, &passkey].map(String::as_str));
                assert!(ca.starts_with("cert-authority,"), "{ca}");
                assert!(device.starts_with("ecdsa-sha2-nistp256 "), "{device}");
                assert!(passkey.starts_with("sk-ecdsa-sha2-nistp256@openssh.com "), "{passkey}");
                assert_eq!(combined, format!("{device}\n{passkey}"));
                scenario_steps::waiting::until("combined public key copy notification", || {
                    Ok(window.notifications()?.iter().any(|message| message == "Device and Passkey Public Keys copied"))
                }).unwrap();
                evidence_text!("Certificate Authority Public Key", &ca);
                evidence_text!("Device + Passkey Public Keys", &combined);
                window.capture_screenshot("Public Keys opened from Profile management").unwrap();
                window.close().unwrap();
            }
        }
    }
}
