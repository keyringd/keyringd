use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::ssh::SshSession;

use crate::scenario_steps;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_system_tests::ssh::{SshServers, SshTestServer};
use keyringd_testkit::{evidence_image, evidence_log, scenario};

scenario! {
    group "SSH Authentication / Passkey over PXP BLE Relay" {
        case "Authenticates SSH commands using a phone or tablet Passkey" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::hybrid_available()
                => "requires AuthenticatorServer and Hybrid tunnel";
            given "Profiles and Real SSH Servers Created with Passkey on Phone" {
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
                let configuration =
                    keyringd_system_tests::application::ApplicationConfiguration {
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
                    "phone-passkey-ssh",
                )
                .unwrap();
                environment
                    .application()
                    .capture_screenshot("Profiles after setup")
                    .unwrap();
                let passkey = scenario_steps::profile_management::public_keys::copy_passkey(
                    &mut environment,
                    "phone-passkey-ssh",
                )
                .unwrap();
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| identity.comment == "keyringd phone-passkey-ssh passkey")
                    .unwrap();
            }
            when "Read the QR on the SSH signature screen with your phone and relay the ad via PXP" {
                assert!(
                    environment.application().uses_native_fido().unwrap(),
                    "Passkey SSH must use native WebAuthn"
                );
                let ssh_server = environment.ssh_servers().start(&passkey).unwrap();
                let ssh = ssh_server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        &identity,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap();
                let (qr, image) =
                    scenario_steps::ssh_authentication::hybrid::read_qr(&mut environment).unwrap();
                let advertisement = phone.scan_hybrid_qr(&qr).unwrap();
                environment
                    .application()
                    .send_hybrid_advertisement(&advertisement)
                    .unwrap();
                let connection = ssh.finish().unwrap();
                scenario_steps::ssh_authentication::prompts::wait_closed(&mut environment).unwrap();
                phone.authenticator().wait_for_no_pending_requests().unwrap();
                evidence_image!("Passkey SSH QR", image);
            }
            then "Phone Passkey is accepted, command is successful and signing request ends" {
                assert!(qr.starts_with("FIDO:/"), "{qr}");
                connection.assert_authenticated(&identity);
                evidence_log!("Phone SSH diagnostics", &connection.stderr);
                evidence_log!("sshd authentication log", &connection.server_log);
            }
        }

        case "Starts a new Hybrid session for each successive FIDO operation" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::hybrid_available()
                => "requires AuthenticatorServer and Hybrid tunnel";
            given "Repeatedly SSH-authenticated profile on Passkey on the same phone" {
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
                let configuration =
                    keyringd_system_tests::application::ApplicationConfiguration {
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
                    "phone-passkey-repeat",
                )
                .unwrap();
                environment
                    .application()
                    .capture_screenshot("Profiles after setup")
                    .unwrap();
                let passkey = scenario_steps::profile_management::public_keys::copy_passkey(
                    &mut environment,
                    "phone-passkey-repeat",
                )
                .unwrap();
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| identity.comment == "keyringd phone-passkey-repeat passkey")
                    .unwrap();
            }
            when "Approve your first SSH signature on your phone" {
                let ssh_server = environment.ssh_servers().start(&passkey).unwrap();
                let ssh = ssh_server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        &identity,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap();
                let (first_qr, first_image) =
                    scenario_steps::ssh_authentication::hybrid::read_qr(&mut environment).unwrap();
                let advertisement = phone.scan_hybrid_qr(&first_qr).unwrap();
                environment
                    .application()
                    .send_hybrid_advertisement(&advertisement)
                    .unwrap();
                let first = ssh.finish().unwrap();
                scenario_steps::ssh_authentication::prompts::wait_closed(&mut environment).unwrap();
                phone.authenticator().wait_for_no_pending_requests().unwrap();
                first.assert_authenticated(&identity);
                evidence_image!("First Passkey SSH QR", first_image);
                evidence_log!("First Phone SSH diagnostics", &first.stderr);
                evidence_log!("First sshd authentication log", &first.server_log);
            }
            when "Approve a second SSH signature on the same phone" {
                let ssh_server = environment.ssh_servers().start(&passkey).unwrap();
                let ssh = ssh_server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        &identity,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap();
                let (second_qr, second_image) =
                    scenario_steps::ssh_authentication::hybrid::read_qr(&mut environment).unwrap();
                let advertisement = phone.scan_hybrid_qr(&second_qr).unwrap();
                environment
                    .application()
                    .send_hybrid_advertisement(&advertisement)
                    .unwrap();
                let second = ssh.finish().unwrap();
                scenario_steps::ssh_authentication::prompts::wait_closed(&mut environment).unwrap();
                phone.authenticator().wait_for_no_pending_requests().unwrap();
                second.assert_authenticated(&identity);
                evidence_image!("Second Passkey SSH QR", second_image);
                evidence_log!("Second Phone SSH diagnostics", &second.stderr);
                evidence_log!("Second sshd authentication log", &second.server_log);
            }
            then "Each request uses a separate Hybrid session with a different QR" {
                assert!(first_qr.starts_with("FIDO:/"), "{first_qr}");
                assert!(second_qr.starts_with("FIDO:/"), "{second_qr}");
                assert_ne!(first_qr, second_qr);
            }
        }
    }
}
