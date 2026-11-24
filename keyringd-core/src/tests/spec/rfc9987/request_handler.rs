use keyringd_ssh_agent_protocol::SshPublicKey;
use std::sync::Arc;

use crate::agent::KeyringdAgentRequestHandler;
use crate::application::KeyringdApplication;
use crate::application::PasskeyChoice;
use crate::application::ProfileSetupRequest;
use crate::model::{ProfileSshIdentity, ProfileSshIdentityKind};
use crate::service::{DeviceKeyService, PasskeyService};
use crate::test_support::MemoryProfileRepository;
use crate::test_support::TestPasskeyService;
use crate::test_support::{MockDeviceKeyService, successful_device_keys};
use keyringd_fido_client::HybridAdvertisementRelay;
use keyringd_ssh_agent_protocol::extension_names::PXP_ADVERTISEMENT_RELAY_EXTENSION;
use keyringd_ssh_agent_protocol::{
    SshAgentExtensionRequest, SshAgentRequest, SshAgentRequestHandler, SshAgentResponse,
    SshAgentSignFlags, SshAgentSignRequest, SshKeyAlgorithm, SshPublicKeyBlob,
    SshSignatureAlgorithm,
};
use p256::elliptic_curve::sec1::ToSec1Point;

async fn completed_application()
-> Arc<KeyringdApplication<MockDeviceKeyService, TestPasskeyService>> {
    let application = KeyringdApplication::new(
        successful_device_keys(),
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

fn append_string(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend((value.len() as u32).to_be_bytes());
    bytes.extend(value);
}

fn sshsig_payload() -> Vec<u8> {
    let mut data = b"SSHSIG".to_vec();
    append_string(&mut data, b"keyringd-test");
    append_string(&mut data, b"");
    append_string(&mut data, b"sha256");
    append_string(&mut data, &[7; 32]);
    data
}

fn userauth_payload(algorithm: &[u8], public_key: &SshPublicKeyBlob) -> Vec<u8> {
    let mut data = Vec::new();
    append_string(&mut data, b"session-id");
    data.push(50);
    append_string(&mut data, b"user");
    append_string(&mut data, b"ssh-connection");
    append_string(&mut data, b"publickey");
    data.push(1);
    append_string(&mut data, algorithm);
    append_string(&mut data, &public_key.to_bytes());
    data
}

fn profile_identity<D: DeviceKeyService + 'static, F: PasskeyService + 'static>(
    application: &KeyringdApplication<D, F>,
    kind: ProfileSshIdentityKind,
) -> SshPublicKeyBlob
where
{
    ProfileSshIdentity::from_profile_at(
        &application.profiles()[0],
        kind,
        std::time::SystemTime::now(),
    )
    .expect("the completed test profile exposes every identity kind")
    .key_blob()
    .clone()
}

keyringd_testkit::scenario! {
    group "RFC 9987 product request handling" {
        case "Rejects a signing request for an unregistered public key" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.6"
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "an Agent handler with one completed profile and an unrelated ECDSA public key" {
                let application = completed_application().await;
                let handler = KeyringdAgentRequestHandler::new(Arc::clone(&application));
                let data = sshsig_payload();
            }
            when "the agent receives a valid SSHSIG signing request for the unrelated public key" {
                let response = handler.handle(SshAgentRequest::Sign(SshAgentSignRequest::new(
                    SshPublicKey::from_ecdsa_pubkey(
                        p256::SecretKey::from_slice(&[9; 32]).unwrap().public_key().to_sec1_point(false).as_bytes(),
                    ).unwrap().into_key_blob(),
                    data,
                    SshAgentSignFlags::default(),
                ))).await;
            }
            then "the agent rejects the request because the key is not registered to any profile" {
                assert!(matches!(response, SshAgentResponse::Failure));
            }
        }

        case "Accepts a userauth signing request that identifies the matching Passkey" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.6"
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "a completed profile and a userauth payload whose algorithm and public-key blob identify its Passkey" {
                let application = completed_application().await;
                let passkey = profile_identity(&application, ProfileSshIdentityKind::Passkey);
                let data = userauth_payload(SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes(), &passkey);
                let handler = KeyringdAgentRequestHandler::new(application);
            }
            when "the agent receives a signing request for that Passkey and payload" {
                let response = handler.handle(SshAgentRequest::Sign(SshAgentSignRequest::new(
                    passkey,
                    data,
                    SshAgentSignFlags::default(),
                ))).await;
            }
            then "the agent accepts the request and returns a Passkey signature" {
                let SshAgentResponse::Signature { signature } = response else {
                    panic!("expected a Passkey signature");
                };
                assert_eq!(signature.algorithm(), SshSignatureAlgorithm::SkEcdsaSha2NistP256);
            }
        }

        case "Rejects every nonzero signature flag on a non-RSA identity" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.6.1"
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-8.3"
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "an Agent handler and a completed profile's non-RSA Device Key identity" {
                let application = completed_application().await;
                let identity = profile_identity(&application, ProfileSshIdentityKind::DeviceKey);
                let handler = KeyringdAgentRequestHandler::new(Arc::clone(&application));
                let data = sshsig_payload();
            }
            when "requests specify each defined RSA flag, their combination, a reserved flag, and an unknown flag" {
                let mut responses = Vec::new();
                for bits in [2, 4, 6, 1, 8] {
                    responses.push(handler.handle(SshAgentRequest::Sign(SshAgentSignRequest::new(
                        identity.clone(), data.clone(), SshAgentSignFlags::from_bits(bits),
                    ))).await);
                }
            }
            then "the agent rejects every incompatible or unsupported flag without signing" {
                assert!(responses.iter().all(|response| matches!(response, SshAgentResponse::Failure)));
            }
        }

        case "Rejects an unsupported session-bind extension request" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.8"
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "an Agent handler with a completed profile" {
                let application = completed_application().await;
                let handler = KeyringdAgentRequestHandler::new(application);
            }
            when "the agent receives a session-bind extension request that it cannot enforce" {
                let response = handler.handle(SshAgentRequest::Extension(SshAgentExtensionRequest::new(
                    "session-bind@openssh.com".to_owned(), Vec::new(),
                ))).await;
            }
            then "the agent returns Failure for the unsupported extension request" {
                assert!(matches!(response, SshAgentResponse::Failure));
            }
        }

        case "Remote Hybrid advertisements require an explicit Agent option" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.8"
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "two Agent handlers sharing an application, only one configured with a remote advertisement relay" {
                let application = completed_application().await;
                let disabled = KeyringdAgentRequestHandler::new(Arc::clone(&application));
                let relay = HybridAdvertisementRelay::new();
                let mut received = relay.subscribe();
                let enabled = KeyringdAgentRequestHandler::new(application)
                    .with_remote_hybrid_advertisements(relay);
                let mut advertisement = vec![0xf9, 0xff];
                advertisement.extend_from_slice(&[0x42; 20]);
            }
            when "the same well-formed advertisement is submitted to both handlers" {
                let disabled_response = disabled.handle(SshAgentRequest::Extension(
                    SshAgentExtensionRequest::new(PXP_ADVERTISEMENT_RELAY_EXTENSION.to_owned(), advertisement.clone())
                )).await;
                let enabled_response = enabled.handle(SshAgentRequest::Extension(
                    SshAgentExtensionRequest::new(PXP_ADVERTISEMENT_RELAY_EXTENSION.to_owned(), advertisement)
                )).await;
                let mut malformed_responses = Vec::new();
                for length in (0..=23).filter(|length| *length != 22) {
                    let mut payload = vec![0x42; length];
                    if length >= 2 {
                        payload[..2].copy_from_slice(&[0xf9, 0xff]);
                    }
                    malformed_responses.push(enabled.handle(SshAgentRequest::Extension(
                        SshAgentExtensionRequest::new(PXP_ADVERTISEMENT_RELAY_EXTENSION.to_owned(), payload)
                    )).await);
                }
                let mut unsupported_service = vec![0x0f, 0x18];
                unsupported_service.extend_from_slice(&[0x42; 20]);
                let unsupported_service_response = enabled.handle(SshAgentRequest::Extension(
                    SshAgentExtensionRequest::new(PXP_ADVERTISEMENT_RELAY_EXTENSION.to_owned(), unsupported_service)
                )).await;
                let mut unsupported_name_advertisement = vec![0xf9, 0xff];
                unsupported_name_advertisement.extend_from_slice(&[0x42; 20]);
                let unsupported_name_response = enabled.handle(SshAgentRequest::Extension(
                    SshAgentExtensionRequest::new("unsupported-extension@keyringd.com".to_owned(), unsupported_name_advertisement)
                )).await;
            }
            then "unsupported extensions return Failure and invalid supported extension payloads return Extension Failure without being relayed" {
                assert!(matches!(disabled_response, SshAgentResponse::Failure));
                assert!(matches!(enabled_response, SshAgentResponse::Success));
                for response in malformed_responses.iter().chain([&unsupported_service_response]) {
                    assert!(matches!(response, SshAgentResponse::ExtensionFailure));
                    assert_eq!(response.to_bytes().unwrap(), vec![28]);
                }
                assert!(matches!(unsupported_name_response, SshAgentResponse::Failure));
                assert_eq!(received.try_recv().unwrap(), [0x42; 20]);
                assert!(received.try_recv().is_err());
            }
        }

        case "Rejects every unsupported or unassigned SSH Agent request type" {
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-5.1"
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-8.1"
            spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-8.1.1"
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "an Agent handler and every message type other than the three requests implemented by keyringd" {
                let application = completed_application().await;
                let handler = KeyringdAgentRequestHandler::new(application);
                let unsupported_types = (0_u8..=u8::MAX)
                    .filter(|message_type| !matches!(message_type, 11 | 13 | 27))
                    .collect::<Vec<_>>();
            }
            when "the handler receives each type as an unsupported request with opaque contents" {
                let mut responses = Vec::new();
                for message_type in &unsupported_types {
                    responses.push(handler.handle(SshAgentRequest::Unknown {
                        message_type: *message_type,
                        data: vec![1, 2, 3],
                    }).await);
                }
            }
            then "every unsupported request returns Failure without dispatching an operation" {
                assert_eq!(responses.len(), unsupported_types.len());
                assert!(responses.iter().all(|response| matches!(response, SshAgentResponse::Failure)));
            }
        }
    }
}
