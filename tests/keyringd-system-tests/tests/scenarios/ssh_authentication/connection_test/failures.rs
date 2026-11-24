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
    group "Test SSH Connection / Failures" {
        case "Distinguishes OpenSSH configuration errors, identity rejection, and connection failures" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available() => "requires Security Key environment";
            given "The connection test screen that selected Device Certificate in Profile" {
                let (mut environment, _key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&[PROFILE])
                    .unwrap();
                let trust = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    PROFILE,
                )
                .unwrap();
                let identity = SshConnectionIdentity::DeviceCertificate;
                environment
                    .application()
                    .ssh_connection_test_window()
                    .open(PROFILE)
                    .unwrap();
                environment
                    .application()
                    .ssh_connection_test_window()
                    .select_identity(identity)
                    .unwrap();
                assert_eq!(
                    environment
                        .application()
                        .ssh_connection_test_window()
                        .selected_identities()
                        .unwrap(),
                    vec![identity]
                );
            }
            when "Connect from the same host setting without using the agent" {
                let mut server = environment.ssh_servers().start(&trust.passkey).unwrap();
                let mut configuration = server.client_configuration().unwrap();
                configuration.use_agent = false;
                environment
                    .application()
                    .set_ssh_client_configuration(HOST, &configuration)
                    .unwrap();
                let configuration_guidance =
                    scenario_steps::ssh_connection_test::configuration::expected_guidance(
                        &mut environment,
                    )
                    .unwrap();
                environment
                    .application()
                    .ssh_connection_test_window()
                    .start(HOST)
                    .unwrap();
                let configuration_result = environment
                    .application()
                    .connection_test_result_window()
                    .content()
                    .unwrap();
            }
            then "Guidance without omitting the SSH setting method of the OS" {
                scenario_steps::ssh_connection_test::result::assert_failure(
                    &configuration_result,
                    HOST,
                    "SSH Configuration Needs Attention",
                    &configuration_guidance,
                );
                environment
                    .application()
                    .connection_test_result_window()
                    .capture_screenshot("Device Certificate SSH configuration failure")
                    .unwrap();
                environment
                    .application()
                    .connection_test_result_window()
                    .close()
                    .unwrap();
                server.stop().unwrap();
            }
            when "Select with the same Host to connect Identity to an untrusted server" {
                let mut server = environment.ssh_servers().start(&trust.passkey).unwrap();
                let configuration = server.client_configuration().unwrap();
                environment
                    .application()
                    .set_ssh_client_configuration(HOST, &configuration)
                    .unwrap();
                environment
                    .application()
                    .ssh_connection_test_window()
                    .start(HOST)
                    .unwrap();
                let rejection = environment
                    .application()
                    .connection_test_result_window()
                    .content()
                    .unwrap();
            }
            then "Identity denial description, server setting guide and correct registration entry" {
                scenario_steps::ssh_connection_test::result::assert_failure(
                    &rejection,
                    HOST,
                    "Device Certificate Rejected",
                    "OpenSSH offered this profile’s Device Certificate, but the server rejected it.",
                );
                scenario_steps::ssh_connection_test::result::assert_rejection_guidance(
                    &rejection, identity, &trust.ca,
                );
                assert_eq!(
                    environment
                        .application()
                        .connection_test_result_window()
                        .copy_rejected_entry()
                        .unwrap(),
                    trust.ca
                );
                environment
                    .application()
                    .connection_test_result_window()
                    .capture_screenshot("Device Certificate rejected identity")
                    .unwrap();
                environment
                    .application()
                    .connection_test_result_window()
                    .close()
                    .unwrap();
                server.stop().unwrap();
                let rejected_log = server.log().unwrap();
            }
            when "Stop and connect the same host server" {
                let mut server = environment.ssh_servers().start(&trust.passkey).unwrap();
                let configuration = server.client_configuration().unwrap();
                server.stop().unwrap();
                environment
                    .application()
                    .set_ssh_client_configuration(HOST, &configuration)
                    .unwrap();
                environment
                    .application()
                    .ssh_connection_test_window()
                    .start(HOST)
                    .unwrap();
                let unreachable = environment
                    .application()
                    .connection_test_result_window()
                    .content()
                    .unwrap();
            }
            then "Differentiate communication failures and use the same Host and Identity for all 3 requests" {
                scenario_steps::ssh_connection_test::result::assert_failure(
                    &unreachable,
                    HOST,
                    "SSH Connection Failed",
                    "Could not connect to the SSH server. Check the host name, network connection, and server settings, then try again.",
                );
                environment
                    .application()
                    .connection_test_result_window()
                    .capture_screenshot("Device Certificate unreachable server")
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
                let requests = environment.application().connection_requests().unwrap();
                scenario_steps::ssh_connection_test::requests::assert_identity_when_recorded(
                    &requests, HOST, identity, 3,
                );
                assert!(
                    !rejected_log.contains("Accepted publickey"),
                    "{rejected_log}"
                );
                evidence_log!("Rejected connection sshd", rejected_log);
            }
        }

        case "Shows and copies the public key registration entry when the Device Key is rejected" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available() => "requires Security Key environment";
            given "The connection test screen that selected the Device Key in Profile" {
                let (mut environment, _key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&[PROFILE])
                    .unwrap();
                let trust = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    PROFILE,
                )
                .unwrap();
                let identity = SshConnectionIdentity::DeviceKey;
                environment
                    .application()
                    .ssh_connection_test_window()
                    .open(PROFILE)
                    .unwrap();
                environment
                    .application()
                    .ssh_connection_test_window()
                    .select_identity(identity)
                    .unwrap();
                assert_eq!(
                    environment
                        .application()
                        .ssh_connection_test_window()
                        .selected_identities()
                        .unwrap(),
                    vec![identity]
                );
            }
            when "Select with the same Host to connect Identity to an untrusted server" {
                let mut server = environment.ssh_servers().start(&trust.passkey).unwrap();
                let configuration = server.client_configuration().unwrap();
                environment
                    .application()
                    .set_ssh_client_configuration(HOST, &configuration)
                    .unwrap();
                environment
                    .application()
                    .ssh_connection_test_window()
                    .start(HOST)
                    .unwrap();
                let rejection = environment
                    .application()
                    .connection_test_result_window()
                    .content()
                    .unwrap();
            }
            then "Identity denial description, server setting guide and correct registration entry" {
                scenario_steps::ssh_connection_test::result::assert_failure(
                    &rejection,
                    HOST,
                    "Device Key Rejected",
                    "OpenSSH offered this profile’s Device Key, but the server rejected it.",
                );
                scenario_steps::ssh_connection_test::result::assert_rejection_guidance(
                    &rejection,
                    identity,
                    &trust.device,
                );
                assert_eq!(
                    environment
                        .application()
                        .connection_test_result_window()
                        .copy_rejected_entry()
                        .unwrap(),
                    trust.device
                );
                environment
                    .application()
                    .connection_test_result_window()
                    .capture_screenshot("Device Key rejected identity")
                    .unwrap();
                environment
                    .application()
                    .connection_test_result_window()
                    .close()
                    .unwrap();
                server.stop().unwrap();
                let rejected_log = server.log().unwrap();
                environment
                    .application()
                    .ssh_connection_test_window()
                    .close()
                    .unwrap();
                let requests = environment.application().connection_requests().unwrap();
                scenario_steps::ssh_connection_test::requests::assert_identity_when_recorded(
                    &requests, HOST, identity, 1,
                );
                assert!(
                    !rejected_log.contains("Accepted publickey"),
                    "{rejected_log}"
                );
                evidence_log!("Rejected connection sshd", rejected_log);
            }
        }
    }
}
