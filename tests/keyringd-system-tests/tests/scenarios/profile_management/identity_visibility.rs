use crate::scenario_steps;
use keyringd_system_tests::application::SystemTestApplication;
#[cfg(unix)]
use keyringd_system_tests::application::features::profiles::ProfilesWindowDriver;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
keyringd_testkit::scenario! {
    group "Profile Management / SSH Agent Identities" {
        case "Hides all identities when a profile is disabled and restores the same keys when re-enabled" {
            given "Create a valid Profile as an empty destination" {
                let profile_name = "identity-visibility";
                let host_name = gethostname::gethostname().to_string_lossy().into_owned();
                let (mut environment, _key) = scenario_steps::profile_setup::profile_fixture::create::<NativeSystemTestEnvironment>(&[profile_name]).unwrap();
                let wait_for_identities = |environment: &mut NativeSystemTestEnvironment, expected_enabled, expected_count| {
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                    loop {
                        let actual = environment.application().agent_identities().unwrap();
                        let enabled = environment.application().profiles_window().unwrap().profile_enabled(profile_name).unwrap();
                        if enabled == expected_enabled && actual.len() == expected_count {
                            break actual;
                        }
                        assert!(std::time::Instant::now() < deadline,
                            "expected enabled={expected_enabled} and {expected_count} Agent identities; observed enabled={enabled}, {actual:?}");
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                };
                let before = wait_for_identities(&mut environment, true, 3);
            }
            then "Valid in the list and the Agent exposes the correct three types of Identity" {
                assert!(environment.application().profiles_window().unwrap().profile_enabled(profile_name).unwrap());
                let mut actual: Vec<_> = before.iter().map(|identity| {
                    assert!(!identity.public_key.is_empty());
                    (identity.algorithm.clone(), identity.comment.clone())
                }).collect();
                actual.sort();
                let mut expected = vec![
                    ("ecdsa-sha2-nistp256-cert-v01@openssh.com".to_owned(), format!("keyringd {profile_name} device certificate")),
                    ("ecdsa-sha2-nistp256".to_owned(), format!("keyringd {profile_name} device key@{host_name}")),
                    ("sk-ecdsa-sha2-nistp256@openssh.com".to_owned(), format!("keyringd {profile_name} passkey")),
                ];
                expected.sort();
                assert_eq!(actual, expected);
                keyringd_testkit::evidence_log!("Enabled Agent identities", format!("{before:?}"));
            }
            when "Disable Profile with Switches in Listing" {
                environment.application().profiles_window().unwrap().toggle_profile(profile_name).unwrap();
            }
            then "Disabled in the list and normal agent response has no Identity" {
                let disabled = wait_for_identities(&mut environment, false, 0);
                assert!(!environment.application().profiles_window().unwrap().profile_enabled(profile_name).unwrap());
                assert!(disabled.is_empty());
                keyringd_testkit::evidence_log!("Disabled Agent identities", format!("{disabled:?}"));
                environment.application().capture_screenshot("Profile disabled and hidden from Agent").unwrap();
            }
            when "Re-enable Profile with Switches in Listing" {
                environment.application().profiles_window().unwrap().toggle_profile(profile_name).unwrap();
            }
            then "The same three types of public keys and Comment are returned after being enabled in the list" {
                let after = wait_for_identities(&mut environment, true, 3);
                assert!(environment.application().profiles_window().unwrap().profile_enabled(profile_name).unwrap());
                assert_eq!(after, before, "re-enabling must restore the same complete identities");
                keyringd_testkit::evidence_log!("Re-enabled Agent identities", format!("{after:?}"));
                environment.application().capture_screenshot("Profile re-enabled with the same Agent identities").unwrap();
            }
        }
    }
}
