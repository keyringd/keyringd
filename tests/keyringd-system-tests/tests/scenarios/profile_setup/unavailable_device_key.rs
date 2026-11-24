use keyringd_system_tests::application::features::profiles::ProfilesWindowDriver;
use keyringd_system_tests::application::{ApplicationConfiguration, SystemTestApplication};
use keyringd_system_tests::environment::{NativeSystemTestEnvironment, SystemTestEnvironment};
use keyringd_testkit::scenario;

scenario! {
    group "Profiles / Device Key availability" {
        case "Opening Profiles explains unavailable hardware and disables profile creation" {
            given "hardware key storage is unavailable only to the application under test" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let configuration = ApplicationConfiguration {
                    device_key_unavailable: true,
                    ..Default::default()
                };
                #[cfg(any(windows, target_os = "macos"))]
                let configuration = ApplicationConfiguration {
                    test_fido_mode: Some(keyringd_core::test_support::TestFidoMode::Success),
                    ..configuration
                };
            }
            when "the Profiles window is opened" {
                environment.application().start(configuration).unwrap();
            }
            then "a persistent banner explains the hardware requirement and creation is disabled" {
                let profiles = environment.application().profiles_window().unwrap();
                assert_eq!(profiles.device_key_warning().unwrap(),
                    "Hardware-backed Device Key storage is unavailable on this computer. Check that TPM or Secure Enclave is supported, enabled, and accessible, then try again.");
                assert!(!profiles.creation_enabled().unwrap());
                profiles.capture_screenshot("Profiles with hardware key unavailable").unwrap();
                assert!(environment.application().stored_profiles().unwrap().is_empty());
            }
            when "the application is reopened with usable hardware" {
                environment.application().stop().unwrap();
                environment.application().start(Default::default()).unwrap();
            }
            then "profile creation becomes available on the Profiles window" {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                while !environment.application().profiles_window().unwrap().creation_enabled().unwrap() {
                    assert!(std::time::Instant::now() < deadline, "profile creation was not enabled");
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
            }
        }
    }
}
