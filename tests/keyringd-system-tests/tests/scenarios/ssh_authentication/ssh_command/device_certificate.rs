use crate::scenario_steps;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_system_tests::ssh::{SshServers, SshSession, SshTestServer};
use keyringd_testkit::{evidence_log, evidence_text, scenario};

scenario! {
    group "SSH Authentication / Device Certificate" {
        case "Authenticates using a Device Certificate when the server trusts its CA" {
            given "A Trust Entry that has been copied from the created Profile and the UI" {
                let (mut environment, key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&["ssh-profile"])
                    .unwrap();
                let ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut environment,
                    "ssh-profile",
                )
                .unwrap();
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| {
                        identity.algorithm == "ecdsa-sha2-nistp256-cert-v01@openssh.com"
                    })
                    .unwrap();
            }
            then "Only the CA is registered on the server and the Agent publishes the target Device Certificate." {
                assert!(
                    ca
                        .starts_with("cert-authority,principals=\"keyringd:"),
                    "{}",
                    ca
                );
                assert_eq!(
                    identity.comment,
                    "keyringd ssh-profile device certificate"
                );
                evidence_text!("Certificate Authority Public Key", &ca);
            }
            when "SSH with Product Agents to Temporary SSHD Trusting CA Only" {
                let requests_before = key.authenticator().request_count().unwrap();
                let connection_server = environment.ssh_servers().start(&ca).unwrap();
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
            then "The certificate of interest is accepted, the command is executed, and the FIDO request does not increase." {
                connection.assert_authenticated(&identity);
                assert_eq!(key.authenticator().request_count().unwrap(), requests_before);
                evidence_log!("Device Certificate SSH diagnostics", &connection.stderr);
                evidence_log!("sshd authentication log", &connection.server_log);
                environment
                    .application()
                    .capture_screenshot("SSH profile")
                    .unwrap();
            }
        }
    }
}
