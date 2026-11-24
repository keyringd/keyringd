use crate::scenario_steps;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::ssh_connection_test::{
    SshConnectionIdentity, SshConnectionTestWindowDriver,
};
use keyringd_system_tests::environment::{NativeSystemTestEnvironment, SystemTestEnvironment};
use keyringd_testkit::scenario;

scenario! {
    group "SSH Connection Test / Device Key" {
        case "Keeps exactly one SSH identity radio button selected" {
            given "Open the connection test screen for profiles with 3 different types of identity" {
                let (mut environment, _key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&["conn-test-radio"])
                    .unwrap();
                environment
                    .application()
                    .ssh_connection_test_window()
                    .open("conn-test-radio")
                    .unwrap();
                assert_eq!(
                    environment
                        .application()
                        .ssh_connection_test_window()
                        .selected_identities()
                        .unwrap(),
                    vec![SshConnectionIdentity::DeviceCertificate]
                );
            }
            when "Select each Identity in order and press the selected button again" {
                for identity in [
                    SshConnectionIdentity::DeviceKey,
                    SshConnectionIdentity::Passkey,
                    SshConnectionIdentity::Passkey,
                    SshConnectionIdentity::DeviceCertificate,
                    SshConnectionIdentity::DeviceKey,
                ] {
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
            }
            then "Only the last selected Device Key is ON" {
                assert_eq!(
                    environment
                        .application()
                        .ssh_connection_test_window()
                        .selected_identities()
                        .unwrap(),
                    vec![SshConnectionIdentity::DeviceKey]
                );
                environment
                    .application()
                    .ssh_connection_test_window()
                    .close()
                    .unwrap();
            }
        }
    }
}
