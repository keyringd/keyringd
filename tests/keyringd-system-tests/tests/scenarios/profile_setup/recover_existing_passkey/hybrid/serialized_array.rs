use crate::scenario_steps::profile_setup::hybrid_profiles;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::environment::{NativeSystemTestEnvironment, SystemTestEnvironment};
use keyringd_test_usbip::authenticator_server::client::{
    AuthenticatorConfiguration, LargeBlobSupport,
};
use keyringd_testkit::scenario;

scenario! {
    case "Recovers without local profile data using attestation from the shared large blob array" {
        skip_if: !keyringd_system_tests::authenticator::Authenticators::hybrid_available() => "Requires a virtual phone and a Hybrid test tunnel";
        given "Create a Profile on a shared array of phones" {
            let mut environment = NativeSystemTestEnvironment::new().unwrap();
            let phone = environment.authenticators().create_hybrid_phone(AuthenticatorConfiguration {
                large_blob_support: LargeBlobSupport::LargeBlobKey,
                ..Default::default()
            }).unwrap();
            #[cfg(target_os = "linux")]
            let _advertisements = environment.testbed().remote_hybrid_advertisements().unwrap();
            environment.application().start(keyringd_system_tests::application::ApplicationConfiguration {
                #[cfg(any(target_os = "macos", windows))]
                accept_remote_hybrid_advertisements: true,
                ..Default::default()
            }).unwrap();
            environment.application().profile_setup_window().open().unwrap();
            hybrid_profiles::create(&mut environment, &phone, "array-source").unwrap();
            let attestation = environment.application().stored_profiles().unwrap()[0]["passkey"]["attestationResponse"].clone();
            assert!(attestation.is_object());
        }
        when "Hold the phone and erase the local profile and restore it with two authentications" {
            environment.application().reset_profiles().unwrap();
            assert!(environment.application().stored_profiles().unwrap().is_empty());
            environment.application().profile_setup_window().open().unwrap();
            hybrid_profiles::recover(&mut environment, &phone, "array-recovered", true).unwrap();
        }
        then "Attestation is saved in the restored Profile" {
            let saved = environment.application().stored_profiles().unwrap();
            assert_eq!(saved.len(), 1);
            assert_eq!(saved[0]["displayName"], "array-recovered");
            assert_eq!(saved[0]["passkey"]["attestationResponse"], attestation);
        }
    }
}
