use crate::scenario_steps;
use keyringd_system_tests::application::SystemTestApplication;
#[cfg(unix)]
use keyringd_system_tests::application::features::profiles::ProfilesWindowDriver;
#[cfg(unix)]
use keyringd_system_tests::application::features::profiles::TrustEntriesWindowDriver;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_system_tests::ssh::{SshServers, SshSession, SshTestServer};
use keyringd_testkit::{evidence_log, evidence_text, scenario};

scenario! {
    group "SSH Authentication / Direct Device Key" {
        case "Authenticates with a registered Device Key without a FIDO operation" {
            given "Device key public key copied from the created Profile and the UI" {
                let (mut environment, key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&["ssh-profile"])
                    .unwrap();
                let device_key = scenario_steps::profile_management::public_keys::copy_device_key(
                    &mut environment,
                    "ssh-profile",
                )
                .unwrap();
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| identity.algorithm == "ecdsa-sha2-nistp256")
                    .unwrap();
                let host = gethostname::gethostname().to_string_lossy().into_owned();
            }
            then "The type of key to be registered, the PROFILE name, and the host name are correct." {
                assert!(
                    device_key.starts_with("ecdsa-sha2-nistp256 "),
                    "{}",
                    device_key
                );
                assert!(
                    device_key
                        .ends_with(&format!(" keyringd ssh-profile device key@{host}")),
                    "{}",
                    device_key
                );
                assert_eq!(
                    identity.comment,
                    format!("keyringd ssh-profile device key@{host}")
                );
                evidence_text!("Device Key Public Key", &device_key);
                {
                    let mut profiles = environment.application().profiles_window().unwrap();
                    let mut public_keys = profiles.open_trust_entries("ssh-profile").unwrap();
                    public_keys
                        .capture_screenshot("Direct SSH Public Keys")
                        .unwrap();
                    public_keys.close().unwrap();
                }
            }
            when "SSH with Product Agent to Temporary SSHD that Trust Only Direct Keys" {
                let requests_before = key.authenticator().request_count().unwrap();
                let connection_server = environment.ssh_servers().start(&device_key).unwrap();
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
            then "Uncertified keys are accepted and commands are executed without increasing FIDO requests" {
                connection.assert_authenticated(&identity);
                assert_eq!(key.authenticator().request_count().unwrap(), requests_before);
                evidence_log!("Device Key SSH diagnostics", &connection.stderr);
                evidence_log!("sshd authentication log", &connection.server_log);
                environment
                    .application()
                    .capture_screenshot("SSH profile")
                    .unwrap();
            }
        }
    }
}
