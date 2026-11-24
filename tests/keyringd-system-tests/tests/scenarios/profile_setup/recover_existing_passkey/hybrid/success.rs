use crate::scenario_steps;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_system_tests::ssh::{SshServers, SshSession, SshTestServer};
use keyringd_testkit::agent::AgentIdentity;
use keyringd_testkit::{evidence_image, evidence_log, evidence_text, scenario};

scenario! {
    case "Recovers a profile using an existing Passkey through the PXP BLE advertisement relay" {
        skip_if: !keyringd_system_tests::authenticator::Authenticators::hybrid_available() => "Requires a virtual phone and a Hybrid test tunnel";
        given "Create the original profile and record public keys on the same phone" {
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
        }
        when "Prepare an empty save destination equivalent to a new PC while leaving the phone credentials" {
            let source_attestation = environment.application().stored_profiles().unwrap()[0]
                ["passkey"]["attestationResponse"].clone();
            assert!(source_attestation.is_object());
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
        when "Respond to two different QRs on the same phone" {
            scenario_steps::profile_setup::hybrid_profiles::recover(
                &mut environment,
                &phone,
                "pxp-recovered",
                true,
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
            assert_eq!(saved[0]["passkey"]["attestationResponse"], source_attestation);
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

#[cfg(target_os = "linux")]
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
    ProfileSetupStep,
};

scenario! {
    #[cfg(target_os = "linux")]
    case "Recovers a profile using an existing Passkey through btvirt and BlueZ" {
        skip_if: std::env::var_os("KEYRINGD_E2E_BLE_ADVERTISER_ADDRESS").is_none()
            => "requires btvirt controllers on a host with hci_vhci";
        given "Create Passkey on Phone and Record Public Keys on Existing PCs" {
            let mut environment = NativeSystemTestEnvironment::new().unwrap();
            let phone = environment
                .authenticators()
                .create_hybrid_phone(Default::default())
                .unwrap();
            let advertiser = environment.testbed().ble_advertiser().unwrap();
            environment.application().start(Default::default()).unwrap();
            environment
                .application()
                .profile_setup_window()
                .open()
                .unwrap();
            scenario_steps::profile_setup::form::submit_creation(
                &mut environment,
                "source-phone-pc",
            )
            .unwrap();
            scenario_steps::profile_setup::transport_choices::assert_creation(&mut environment)
                .unwrap();
            let (qr1, image1) = environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .interaction_page()
                .unwrap()
                .qr_view()
                .unwrap()
                .read_hybrid_qr(None)
                .unwrap();
            evidence_image!("Source Passkey QR", image1);
            let first_advertisement = phone.scan_hybrid_qr(&qr1).unwrap();
            advertiser.advertise(&first_advertisement).unwrap();
            scenario_steps::profile_setup::progress::wait_for_step(
                &mut environment,
                ProfileSetupStep::AuthorizeNewDevice,
            )
            .unwrap();
            let (qr2, image2) = environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .interaction_page()
                .unwrap()
                .qr_view()
                .unwrap()
                .read_hybrid_qr(Some(&qr1))
                .unwrap();
            assert_ne!(qr1, qr2);
            evidence_image!("Source Device Key local authentication QR", image2);
            let second_advertisement = phone.scan_hybrid_qr(&qr2).unwrap();
            advertiser.advertise(&second_advertisement).unwrap();
            environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .completion_page()
                .unwrap()
                .done()
                .unwrap();
            let source = scenario_steps::profile_management::public_keys::read(
                &mut environment,
                "source-phone-pc",
            )
            .unwrap();
        }
        when "Start recovery of existing Passkey" {
            environment.application().reset_profiles().unwrap();
            environment
                .application()
                .profile_setup_window()
                .open()
                .unwrap();
            scenario_steps::profile_setup::form::submit_recovery(
                &mut environment,
                "recovered-phone-pc",
            )
            .unwrap();
        }
        then "Shows how to find existing Passkey" {
            scenario_steps::profile_setup::transport_choices::assert_recovery(&mut environment)
                .unwrap();
        }
        when "Erase only PC status, read two new QRs on the same phone and recover via Btvirt" {
            let (find_qr, find_image) = environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .interaction_page()
                .unwrap()
                .qr_view()
                .unwrap()
                .read_hybrid_qr(None)
                .unwrap();
            evidence_image!("Find existing Passkey QR", find_image);
            let advertisement = phone.scan_hybrid_qr(&find_qr).unwrap();
            advertiser.advertise(&advertisement).unwrap();
        }
        then "The public key is restored from the largeBlob and the device key is being approved." {
            scenario_steps::profile_setup::progress::wait_for_step(
                &mut environment,
                ProfileSetupStep::AuthorizeRecoveredDevice,
            )
            .unwrap();
        }
        when "Erase only PC status, read two new QRs on the same phone and recover via Btvirt" {
            let (authorize_qr, authorize_image) = environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .interaction_page()
                .unwrap()
                .qr_view()
                .unwrap()
                .read_hybrid_qr(Some(&find_qr))
                .unwrap();
        }
        then "The QR for each recovery operation is different from the previous one." {
            assert_ne!(find_qr, authorize_qr);
            evidence_image!("Authorize recovered Device Key QR", authorize_image);
        }
        when "Erase only PC status, read two new QRs on the same phone and recover via Btvirt" {
            let advertisement = phone.scan_hybrid_qr(&authorize_qr).unwrap();
            advertiser.advertise(&advertisement).unwrap();
        }
        then "Profiles are created with the same CA and new Device Key via Bluez/BtlePlug" {
            scenario_steps::profile_setup::completion::wait_for(&mut environment).unwrap();
            environment
                .application()
                .capture_screenshot("Recovered Phone profile over btvirt")
                .unwrap();
            let journal = environment.application().agent_journal().unwrap();
            evidence_log!("BlueZ/btleplug Hybrid recovery", &journal);
            environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .completion_page()
                .unwrap()
                .done()
                .unwrap();
            let recovered = scenario_steps::profile_management::public_keys::read(
                &mut environment,
                "recovered-phone-pc",
            )
            .unwrap();
            assert_eq!(recovered.ca, source.ca);
            assert_ne!(recovered.device, source.device);
            evidence_text!("Preserved Certificate Authority Public Key", &recovered.ca);
            evidence_text!("New Device Key Public Key", &recovered.device);
        }
        when "Connect to the SSH server that registered only the Certificate Authority Public Key of the original PC" {
            let identity = environment
                .application()
                .agent_identities()
                .unwrap()
                .into_iter()
                .find(|identity| identity.algorithm == "ecdsa-sha2-nistp256-cert-v01@openssh.com")
                .unwrap();
            let server = environment.ssh_servers().start(&source.ca).unwrap();
            let connection = server
                .start_client(
                    &environment.application().agent_endpoint().unwrap(),
                    &identity,
                    "echo keyringd-ssh-ok",
                )
                .unwrap()
                .finish()
                .unwrap();
        }
        then "You can authenticate with a new PC certificate without changing the server settings" {
            connection.assert_authenticated(&identity);
            evidence_log!("Recovered Phone SSH diagnostics", &connection.stderr);
            evidence_log!("sshd authentication log", &connection.server_log);
        }
    }
}
