use crate::scenario_steps;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profiles::{
    ProfileDetailsWindowDriver, ProfilesWindowDriver, ReplaceDeviceKeyWindowDriver,
    TrustEntriesWindowDriver,
};
use keyringd_system_tests::authenticator::ApprovalMode;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_system_tests::ssh::{SshServers, SshSession, SshTestServer};
use keyringd_testkit::agent::AgentIdentity;
use keyringd_testkit::{evidence_log, evidence_text, scenario};

const PROFILE: &str = "replace-device-key";

scenario! {
    group "Profile Management / Device Key Replacement" {
        case "Authenticates with the existing CA and the new Device Key after replacement" {
            given "Record the CA and Device Key of the Created Profile" {
                let (mut environment, key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&[PROFILE])
                    .unwrap();
                let original = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    PROFILE,
                )
                .unwrap();
                let original_handle =
                    environment.application().stored_profiles().unwrap()[0]["deviceKey"]["handle"]
                        .clone();
                assert!(original_handle.is_string());
            }
            when "Approve replacement of Device Key from UI" {
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                let mut profiles = environment.application().profiles_window().unwrap();
                let mut details_window = profiles.open_details(PROFILE).unwrap();
                let mut replacement_window = details_window.open_replacement().unwrap();
                replacement_window
                    .capture_screenshot("Replace Device Key confirmation")
                    .unwrap();
                replacement_window.confirm().unwrap();
                replacement_window.press_use_security_key().unwrap();
                scenario_steps::profile_management::device_key_replacement::approve(
                    &replacement_window,
                    &key,
                )
                .unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Automatic).unwrap();
                let result = replacement_window.result().unwrap();
            }
            then "A description of the completion of replacement and a public keys close operation is displayed" {
                for message in [
                    "Device Key Replaced",
                    "Replace the old Device Key public key wherever you registered it. Open Public Keys… to copy the new key.\n\nExisting Passkey and CA registrations remain valid.",
                ] {
                    assert!(
                        result.messages.iter().any(|actual| actual == message),
                        "missing replacement message {message:?}: {:?}",
                        result.messages
                    );
                }
                assert!(result.public_keys_enabled);
                assert!(result.close_enabled);
                replacement_window
                    .capture_screenshot("Device Key replaced")
                    .unwrap();
                let mut public_keys = replacement_window.open_trust_entries(PROFILE).unwrap();
                public_keys.close().unwrap();
                details_window.close().unwrap();
            }
            then "Keep CA and Passkey and update only Device Key" {
                scenario_steps::profile_management::profile_repository::wait_for(
                    &mut environment,
                    |profiles| {
                        profiles.len() == 1 && profiles[0]["deviceKey"]["handle"] != original_handle
                    },
                )
                .unwrap();
                let replacement = scenario_steps::profile_management::public_keys::read(
                    &mut environment,
                    PROFILE,
                )
                .unwrap();
                assert_eq!(replacement.ca, original.ca);
                assert_eq!(
                    AgentIdentity::from_openssh(&replacement.passkey)
                        .unwrap()
                        .public_key,
                    AgentIdentity::from_openssh(&original.passkey)
                        .unwrap()
                        .public_key
                );
                assert_ne!(
                    AgentIdentity::from_openssh(&replacement.device)
                        .unwrap()
                        .public_key,
                    AgentIdentity::from_openssh(&original.device)
                        .unwrap()
                        .public_key
                );
                let identities = environment.application().agent_identities().unwrap();
                let certificate = identities
                    .iter()
                    .find(|identity| {
                        identity.comment == format!("keyringd {PROFILE} device certificate")
                    })
                    .unwrap();
                let host = gethostname::gethostname().to_string_lossy().into_owned();
                let device = identities
                    .iter()
                    .find(|identity| {
                        identity.comment == format!("keyringd {PROFILE} device key@{host}")
                    })
                    .unwrap();
                assert_eq!(
                    device.public_key,
                    AgentIdentity::from_openssh(&replacement.device)
                        .unwrap()
                        .public_key
                );
                let requests_before = key.authenticator().request_count().unwrap();
                evidence_text!("Replacement Device Key", &replacement.device);
            }
            when "Connect existing CA and new Device Key to trusted servers" {
                let ca_connection_server = environment.ssh_servers().start(&original.ca).unwrap();
                let ca_connection = ca_connection_server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        certificate,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap()
                    .finish()
                    .unwrap();
                let direct_connection_server = environment
                    .ssh_servers()
                    .start(&replacement.device)
                    .unwrap();
                let direct_connection = direct_connection_server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        device,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap()
                    .finish()
                    .unwrap();
            }
            then "Can be SSH-authenticated with Substitution Certificate and Device Key" {
                ca_connection.assert_authenticated(certificate);
                direct_connection.assert_authenticated(device);
                evidence_log!("Replacement CA SSH", &ca_connection.stderr);
                evidence_log!("CA sshd log", &ca_connection.server_log);
                evidence_log!("Replacement Device Key SSH", &direct_connection.stderr);
                evidence_log!("Device Key sshd log", &direct_connection.server_log);
            }
            when "Show only new direct keys to servers that trust only the old Device Key" {
                let rejected_server = environment.ssh_servers().start(&original.device).unwrap();
                let rejected = rejected_server
                    .start_client(
                        &environment.application().agent_endpoint().unwrap(),
                        device,
                        "echo keyringd-ssh-ok",
                    )
                    .unwrap()
                    .finish()
                    .unwrap();
            }
            then "Authentication is denied, commands are not executed, and FIDO requests do not increase" {
                assert!(!rejected.status.success());
                assert!(rejected.stdout.is_empty(), "{}", rejected.stdout);
                assert!(
                    rejected.stderr.contains("Permission denied (publickey)"),
                    "{}",
                    rejected.stderr
                );
                assert!(
                    !rejected.server_log.contains("Accepted publickey"),
                    "{}",
                    rejected.server_log
                );
                assert_eq!(key.authenticator().request_count().unwrap(), requests_before);
                evidence_log!("Old Device Key rejected", &rejected.stderr);
            }
        }
    }
}
