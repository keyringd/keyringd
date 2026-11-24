use std::sync::Arc;

use crate::agent::KeyringdAgentRequestHandler;
use crate::application::KeyringdApplication;
use crate::application::PasskeyChoice;
use crate::application::ProfileSetupRequest;
use keyringd_ssh_agent_protocol::{SshAgentConfig, SshAgentConnection, SshPublicKeyBlob};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::test_support::MemoryProfileRepository;
use crate::test_support::TestPasskeyService;

use crate::test_support::MockDeviceKeyService;

fn read_string<'a>(bytes: &mut &'a [u8]) -> &'a [u8] {
    let length = u32::from_be_bytes(
        bytes[..4]
            .try_into()
            .expect("an SSH string starts with a four-byte length"),
    ) as usize;
    let value = &bytes[4..4 + length];
    *bytes = &bytes[4 + length..];
    value
}

async fn application_with_profile()
-> Arc<KeyringdApplication<MockDeviceKeyService, TestPasskeyService>> {
    let application = KeyringdApplication::new(
        crate::test_support::successful_device_keys(),
        TestPasskeyService::new(),
        Arc::new(MemoryProfileRepository::default()),
    )
    .unwrap();
    application
        .setup_profile(
            &Default::default(),
            &Default::default(),
            ProfileSetupRequest::new("Default".to_owned(), PasskeyChoice::CreateNew),
            |_| {},
            crate::test_support::reject_credential_selection,
        )
        .await
        .unwrap();
    application
}

keyringd_testkit::scenario! {
    case "Returns a completed profile's identities in SSH Agent wire format over a connection" {
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.5"
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "an application with one completed profile and an in-memory connection between an SSH Agent client and server" {
            let application = application_with_profile().await;
            let profile = application.profiles().remove(0);
            let expected_keys = [
                profile.device_key().certificate().to_key_blob(),
                profile.device_key().certificate().public_key().into_key_blob(),
                profile.passkey().to_ssh_public_key().into_key_blob(),
            ];
            let (mut client, server) = tokio::io::duplex(4096);
            let handler = KeyringdAgentRequestHandler::new(application);
            let task = tokio::spawn(SshAgentConnection::new(server, handler, SshAgentConfig::default()).serve());
        }
        when "the client sends an identities request and reads the framed SSH Agent response" {
            client.write_all(&[0, 0, 0, 1, 11]).await.unwrap();
            let mut length = [0; 4];
            client.read_exact(&mut length).await.unwrap();
            let mut response = vec![0; u32::from_be_bytes(length) as usize];
            client.read_exact(&mut response).await.unwrap();
        }
        then "the wire response contains the profile's certificate, Device Key, and Passkey identities" {
            assert_eq!(response[0], 12);
            assert_eq!(u32::from_be_bytes(response[1..5].try_into().unwrap()), 3);
            let mut identities = &response[5..];
            let device_comment = format!(
                "keyringd Default device key@{}",
                gethostname::gethostname().to_string_lossy()
            );
            for ((expected_algorithm, expected_comment), expected_key) in [
                (b"ecdsa-sha2-nistp256-cert-v01@openssh.com".as_slice(), b"keyringd Default device certificate".as_slice()),
                (b"ecdsa-sha2-nistp256".as_slice(), device_comment.as_bytes()),
                (b"sk-ecdsa-sha2-nistp256@openssh.com".as_slice(), b"keyringd Default passkey".as_slice()),
            ].into_iter().zip(&expected_keys) {
                let key_blob = read_string(&mut identities);
                let public_key = SshPublicKeyBlob::from_bytes(key_blob)
                    .expect("the Agent must return a standard SSH public-key blob");
                assert_eq!(public_key.algorithm().as_bytes(), expected_algorithm);
                assert_eq!(&public_key, expected_key);
                assert_eq!(read_string(&mut identities), expected_comment);
            }
            assert!(identities.is_empty());
            drop(client);
            task.await.unwrap().unwrap();
        }
    }
}
