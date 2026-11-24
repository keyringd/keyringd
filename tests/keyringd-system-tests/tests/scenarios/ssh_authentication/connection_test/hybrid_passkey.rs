use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;

use crate::scenario_steps;
use keyringd_system_tests::application::features::qr_prompt_window::QrPromptWindowDriver;
use keyringd_system_tests::application::features::ssh_connection_test::ConnectionTestResultWindowDriver;
use keyringd_system_tests::application::features::ssh_connection_test::{
    SshConnectionIdentity, SshConnectionTestWindowDriver,
};
use keyringd_system_tests::environment::{NativeSystemTestEnvironment, SystemTestEnvironment};
use keyringd_system_tests::ssh::{SshServers, SshTestServer};
use keyringd_testkit::{evidence_image, evidence_log, scenario};

const PROFILE: &str = "ssh-profile";
const HOST: &str = "ui-test-host";

scenario! {
    group "Test SSH Connection / Passkey over PXP BLE Relay" {
        case "Tests a successful SSH connection using a phone or tablet Passkey over Hybrid" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::hybrid_available() => "requires AuthenticatorServer and Hybrid tunnel";
            given "Real sshd that trusts only Profile and Passkey created in the UI" {
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
                    PROFILE,
                )
                .unwrap();
                environment
                    .application()
                    .capture_screenshot("Profiles after setup")
                    .unwrap();
                let passkey = scenario_steps::profile_management::public_keys::copy_passkey(
                    &mut environment,
                    PROFILE,
                )
                .unwrap();
                let mut server = environment.ssh_servers().start(&passkey).unwrap();
                let configuration = server.client_configuration().unwrap();
                environment
                    .application()
                    .set_ssh_client_configuration(HOST, &configuration)
                    .unwrap();
            }
            when "Select Identity on the connection test screen and start the connection" {
                environment
                    .application()
                    .ssh_connection_test_window()
                    .open(PROFILE)
                    .unwrap();
                environment
                    .application()
                    .ssh_connection_test_window()
                    .select_identity(SshConnectionIdentity::Passkey)
                    .unwrap();
                assert_eq!(
                    environment
                        .application()
                        .ssh_connection_test_window()
                        .selected_identities()
                        .unwrap(),
                    vec![SshConnectionIdentity::Passkey]
                );
                environment
                    .application()
                    .ssh_connection_test_window()
                    .capture_screenshot("Hybrid connection form")
                    .unwrap();
                environment
                    .application()
                    .ssh_connection_test_window()
                    .start(HOST)
                    .unwrap();
            }
            when "Read connection test QR on Phone and approve real SSH signatures" {
                let (qr, image) = environment
                    .application()
                    .qr_prompt_window()
                    .unwrap()
                    .read_hybrid_qr()
                    .unwrap();
                assert!(qr.starts_with("FIDO:/"), "invalid Hybrid QR: {qr}");
                evidence_image!("Hybrid connection QR", image);
                let advertisement = phone.scan_hybrid_qr(&qr).unwrap();
                environment
                    .application()
                    .send_hybrid_advertisement(&advertisement)
                    .unwrap();
            }
            when "Get connection results and actual SSHD records and close the screen" {
                let result = environment
                    .application()
                    .connection_test_result_window()
                    .content()
                    .unwrap();
                environment
                    .application()
                    .connection_test_result_window()
                    .capture_screenshot("Hybrid connection result")
                    .unwrap();
                environment
                    .application()
                    .connection_test_result_window()
                    .close()
                    .unwrap();
                environment
                    .application()
                    .ssh_connection_test_window()
                    .close()
                    .unwrap();
                server.stop().unwrap();
                let log = server.log().unwrap();
                let requests = environment.application().connection_requests().unwrap();
            }
            then "Selected Identity success is displayed and is authenticated with a real SSHD" {
                scenario_steps::ssh_connection_test::result::assert_success(
                    &result,
                    HOST,
                    SshConnectionIdentity::Passkey,
                );
                assert!(log.contains("Accepted publickey"), "{log}");
                scenario_steps::ssh_connection_test::requests::assert_identity_when_recorded(
                    &requests,
                    HOST,
                    SshConnectionIdentity::Passkey,
                    1,
                );
                evidence_log!("Connection test sshd", log);
            }
        }
    }
}
