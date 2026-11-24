use crate::scenario_steps;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_system_tests::ssh::{SshServers, SshSession, SshTestServer};
use keyringd_test_usbip::authenticator_server::client::{
    AuthenticatorConfiguration, LargeBlobSupport,
};
use keyringd_testkit::agent::AgentIdentity;
use keyringd_testkit::{evidence_log, evidence_text, scenario};

scenario! {
    case "Recovers a profile in three authentications without largeBlob support" {
        skip_if: !keyringd_system_tests::authenticator::Authenticators::hybrid_available() => "Requires a virtual phone and a Hybrid test tunnel";
        given "Create the original profile and record public keys on the same phone" {
            let mut environment = NativeSystemTestEnvironment::new().unwrap();
            let phone = environment
                .authenticators()
                .create_hybrid_phone(
                    AuthenticatorConfiguration {
                        large_blob_support: LargeBlobSupport::Unsupported,
                        ..Default::default()
                    },
                )
                .unwrap();
            #[cfg(target_os = "linux")]
            let _advertisements = environment
                .testbed()
                .remote_hybrid_advertisements()
                .unwrap();
            let configuration = keyringd_system_tests::application::ApplicationConfiguration {
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
            scenario_steps::profile_setup::hybrid_profiles::create(
                &mut environment,
                &phone,
                "pxp-source",
            )
            .unwrap();
            let source = scenario_steps::profile_management::public_keys::read(
                &mut environment,
                "pxp-source",
            )
            .unwrap();
            environment
                .application()
                .capture_screenshot("Profile setup result")
                .unwrap();
            let source_attestation = environment.application().stored_profiles().unwrap()[0]
                ["passkey"]["attestationResponse"].clone();
            assert!(source_attestation.is_object());
        }
        when "Prepare an empty save destination equivalent to a new PC while leaving the phone credentials" {
            environment.application().reset_profiles().unwrap();
            environment
                .application()
                .profile_setup_window()
                .open()
                .unwrap();
        }
        then "The original PC's Profile and Identity do not exist on the new PC" {
            assert!(
                environment
                    .application()
                    .stored_profiles()
                    .unwrap()
                    .is_empty()
            );
            assert!(
                environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .is_empty()
            );
        }
        when "Respond to 3 different QRs on the same phone" {
            scenario_steps::profile_setup::hybrid_profiles::recover(
                &mut environment,
                &phone,
                "pxp-recovered",
                false,
            )
            .unwrap();
            environment
                .application()
                .capture_screenshot("Profile setup result")
                .unwrap();
            let recovered = scenario_steps::profile_management::public_keys::read(
                &mut environment,
                "pxp-recovered",
            )
            .unwrap();
        }
        then "Only one profile with a new Device Key independent of the original CA is saved" {
            assert_eq!(recovered.ca, source.ca);
            assert_ne!(
                AgentIdentity::from_openssh(&recovered.device)
                    .unwrap()
                    .public_key,
                AgentIdentity::from_openssh(&source.device)
                    .unwrap()
                    .public_key
            );
            let saved = environment.application().stored_profiles().unwrap();
            assert!(saved[0]["passkey"]["attestationResponse"].is_null());
            assert_eq!(saved.len(), 1);
            assert_eq!(saved[0]["displayName"], "pxp-recovered");
            evidence_text!("Preserved Certificate Authority Public Key", &recovered.ca);
            evidence_text!("New Device Key Public Key", &recovered.device);
        }
        when "Connect to the SSH server that trusts only the original CA with the new PC's certificate" {
            let identity = environment
                .application()
                .agent_identities()
                .unwrap()
                .into_iter()
                .find(|identity| identity.algorithm == "ecdsa-sha2-nistp256-cert-v01@openssh.com")
                .unwrap();
            let connection_server = environment.ssh_servers().start(&source.ca).unwrap();
            let connection = connection_server
                .start_client(
                    &environment.application().agent_endpoint().unwrap(),
                    &identity,
                    "echo keyringd-ssh-ok",
                )
                .unwrap()
                .finish()
                .unwrap();
        }
        then "The command is executed with the certificate of the profile that has been restored without changing the server settings." {
            assert_eq!(
                identity.comment,
                "keyringd pxp-recovered device certificate"
            );
            connection.assert_authenticated(&identity);
            evidence_log!(
                "Recovered Hybrid Device Certificate SSH",
                &connection.stderr
            );
            evidence_log!("sshd authentication log", &connection.server_log);
        }
    }
}
