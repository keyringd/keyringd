use crate::scenario_steps;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::ssh_connection_test::ConnectionTestResultWindowDriver;
use keyringd_system_tests::application::features::ssh_connection_test::{
    SshConnectionIdentity, SshConnectionTestWindowDriver,
};
use keyringd_system_tests::environment::{NativeSystemTestEnvironment, SystemTestEnvironment};
use keyringd_system_tests::ssh::{SshServers, SshTestServer};
use keyringd_testkit::{evidence_log, scenario};

const PROFILE: &str = "ssh-profile";
const HOST: &str = "ui-test-host";

scenario! {
    group "Test SSH Connection" {
        case "Tests a successful SSH connection using the profile’s Device Certificate" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available() => "requires Security Key environment";
            given "A UI-created Profile and a real sshd that trusts only the CA" {
                let (mut environment, _key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&[PROFILE])
                    .unwrap();
                let ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut environment,
                    PROFILE,
                )
                .unwrap();
                let mut server = environment.ssh_servers().start(&ca).unwrap();
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
                assert_eq!(
                    environment
                        .application()
                        .ssh_connection_test_window()
                        .selected_identities()
                        .unwrap(),
                    vec![SshConnectionIdentity::DeviceCertificate]
                );
                environment
                    .application()
                    .ssh_connection_test_window()
                    .capture_screenshot("Device Certificate connection form")
                    .unwrap();
                environment
                    .application()
                    .ssh_connection_test_window()
                    .start(HOST)
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
                    .capture_screenshot("Device Certificate connection result")
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
                    SshConnectionIdentity::DeviceCertificate,
                );
                assert!(log.contains("Accepted publickey"), "{log}");
                scenario_steps::ssh_connection_test::requests::assert_identity_when_recorded(
                    &requests,
                    HOST,
                    SshConnectionIdentity::DeviceCertificate,
                    1,
                );
                evidence_log!("Connection test sshd", log);
            }
        }
    }
}
