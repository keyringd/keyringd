use crate::model::ssh_trust::SshDeviceKeyTrustEntry;
use std::sync::Arc;

use keyringd_ssh_agent_protocol::{SshAgentRequest, SshAgentRequestHandler, SshAgentResponse};

use crate::agent::KeyringdAgentRequestHandler;
use crate::application::KeyringdApplication;
use crate::application::PasskeyChoice;
use crate::application::ProfileSetupRequest;

use crate::test_support::MemoryProfileRepository;
use crate::test_support::TestPasskeyService;

use crate::test_support::SoftwareDeviceKey;

async fn application_with_profiles(
    names: &[&str],
) -> Arc<KeyringdApplication<SoftwareDeviceKey, TestPasskeyService>> {
    let application = KeyringdApplication::new(
        SoftwareDeviceKey::generating(),
        TestPasskeyService::new(),
        Arc::new(MemoryProfileRepository::default()),
    )
    .unwrap();
    for name in names {
        application
            .setup_profile(
                &Default::default(),
                &Default::default(),
                ProfileSetupRequest::new((*name).to_owned(), PasskeyChoice::CreateNew),
                |_| {},
                crate::test_support::reject_credential_selection,
            )
            .await
            .unwrap();
    }
    application
}

keyringd_testkit::scenario! {
    group "SSH Agent identity listing" {
        case "Orders the SSH Agent identity list by identity kind before profile order" {
            why "stable product-defined ordering keeps profile identities predictable even though RFC 9987 does not prescribe their order"
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "an Agent handler with completed profiles named First and Second" {
                let application = application_with_profiles(&["First", "Second"]).await;
                let profiles = application.profiles();
                let first = &profiles[0];
                let second = &profiles[1];
                let expected_keys = [
                    first.device_key().certificate().to_key_blob(),
                    second.device_key().certificate().to_key_blob(),
                    first.device_key().certificate().public_key().into_key_blob(),
                    second.device_key().certificate().public_key().into_key_blob(),
                    first.passkey().to_ssh_public_key().into_key_blob(),
                    second.passkey().to_ssh_public_key().into_key_blob(),
                ];
                assert_ne!(expected_keys[2], expected_keys[3]);
                let device_trust_entries = application.profiles().iter()
                    .map(|profile| SshDeviceKeyTrustEntry::from_profile(profile).to_string())
                    .collect::<Vec<_>>();
                let handler = KeyringdAgentRequestHandler::new(application);
            }
            when "the handler receives an SSH Agent identities request" {
                let response = handler.handle(SshAgentRequest::Identities).await;
            }
            then "the response lists both certificates, both device keys, and both passkeys in that identity-kind order" {
                let SshAgentResponse::IdentitiesAnswer { identities } = response else {
                    panic!("expected an SSH Agent identities answer");
                };
                let comments = identities
                    .iter()
                    .map(|identity| identity.comment())
                    .collect::<Vec<_>>();
                assert_eq!(
                    identities.iter().map(|identity| identity.key_blob()).collect::<Vec<_>>(),
                    expected_keys.iter().collect::<Vec<_>>()
                );
                let hostname = gethostname::gethostname();
                let hostname = hostname.to_string_lossy();
                let first_device_comment = format!("keyringd First device key@{hostname}");
                let second_device_comment = format!("keyringd Second device key@{hostname}");
                assert_eq!(
                    comments,
                    [
                        "keyringd First device certificate",
                        "keyringd Second device certificate",
                        &first_device_comment,
                        &second_device_comment,
                        "keyringd First passkey",
                        "keyringd Second passkey",
                    ]
                );
                for (entry, comment) in device_trust_entries.iter().zip(&comments[2..4]) {
                    assert!(entry.ends_with(&format!(" {comment}")));
                }
            }
        }
    }
}
