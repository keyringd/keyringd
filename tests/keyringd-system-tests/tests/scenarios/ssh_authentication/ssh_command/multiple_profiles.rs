use crate::scenario_steps;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_system_tests::ssh::{SshServers, SshSession, SshTestServer};
use keyringd_testkit::{evidence_log, evidence_text, scenario};

scenario! {
    group "SSH Authentication / Multiple Profiles" {
        case "Authenticates with distinct profile CAs on the servers that trust them" {
            given "Two Profiles with Different CAs and Certificates Published by Agents" {
                let (mut environment, key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&["multi-first", "multi-second"])
                    .unwrap();
                let first_ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut environment,
                    "multi-first",
                )
                .unwrap();
                let second_ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut environment,
                    "multi-second",
                )
                .unwrap();
                let certificates: Vec<_> = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .filter(|identity| {
                        identity.algorithm == "ecdsa-sha2-nistp256-cert-v01@openssh.com"
                    })
                    .collect();
                assert_eq!(certificates.len(), 2);
                let first_certificate = certificates
                    .iter()
                    .find(|identity| {
                        identity.comment == "keyringd multi-first device certificate"
                    })
                    .unwrap();
                let second_certificate = certificates
                    .iter()
                    .find(|identity| {
                        identity.comment == "keyringd multi-second device certificate"
                    })
                    .unwrap();
                assert_ne!(first_ca, second_ca);
                let requests_before = key.authenticator().request_count().unwrap();
                evidence_text!("First Certificate Authority", &first_ca);
                evidence_text!("Second Certificate Authority", &second_ca);
            }
            when "Connect to a server that trusts each CA using an Agent" {
                let first_connection_server = environment.ssh_servers().start(&first_ca).unwrap();
                let first_connection = first_connection_server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        first_certificate,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap()
                    .finish()
                    .unwrap();
                let second_connection_server = environment.ssh_servers().start(&second_ca).unwrap();
                let second_connection = second_connection_server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        second_certificate,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap()
                    .finish()
                    .unwrap();
            }
            then "Each Profile's Certificate is accepted and the command is executed" {
                first_connection.assert_authenticated(first_certificate);
                second_connection.assert_authenticated(second_certificate);
                evidence_log!("First profile SSH diagnostics", &first_connection.stderr);
                evidence_log!("First sshd log", &first_connection.server_log);
                evidence_log!("Second profile SSH diagnostics", &second_connection.stderr);
                evidence_log!("Second sshd log", &second_connection.server_log);
            }
            when "Present only a separate Profile Certificate to each trusted destination" {
                let rejected_first_server = environment.ssh_servers().start(&first_ca).unwrap();
                let rejected_first = rejected_first_server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        second_certificate,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap()
                    .finish()
                    .unwrap();
                let rejected_second_server = environment.ssh_servers().start(&second_ca).unwrap();
                let rejected_second = rejected_second_server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        first_certificate,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap()
                    .finish()
                    .unwrap();
            }
            then "Authentication is denied on both, commands are not executed, and FIDO requests do not increase" {
                for output in [&rejected_first, &rejected_second] {
                    assert!(!output.status.success(), "wrong CA must be rejected");
                    assert!(output.stdout.is_empty(), "{}", output.stdout);
                    assert!(
                        output.stderr.contains("Permission denied (publickey)"),
                        "{}",
                        output.stderr
                    );
                    assert!(
                        !output.server_log.contains("Accepted publickey"),
                        "{}",
                        output.server_log
                    );
                }
                assert_eq!(key.authenticator().request_count().unwrap(), requests_before);
                evidence_log!("Wrong Certificate at first server", &rejected_first.stderr);
                evidence_log!(
                    "Wrong Certificate at second server",
                    &rejected_second.stderr
                );
                environment
                    .application()
                    .capture_screenshot("Multiple SSH profiles")
                    .unwrap();
            }
        }
    }
}
