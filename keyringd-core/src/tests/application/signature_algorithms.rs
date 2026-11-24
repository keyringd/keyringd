use crate::model::ssh_trust::{SshCertificateAuthorityTrustEntry, SshDeviceKeyTrustEntry};
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;
use std::sync::Arc;

use crate::agent::KeyringdAgentRequestHandler;
use crate::application::KeyringdApplication;
use crate::application::PasskeyChoice;
use crate::application::ProfileSetupRequest;
use crate::model::{ProfileSshIdentity, ProfileSshIdentityKind};
use keyringd_ssh_agent_protocol::{
    SshAgentRequest, SshAgentRequestHandler, SshAgentResponse, SshAgentSignFlags,
    SshAgentSignRequest, SshCertificate, SshKeyAlgorithm, SshSignature,
};

use crate::test_support::MemoryProfileRepository;
use crate::test_support::{SoftwareDeviceKey, TestPasskeyService};

fn write_string(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value);
}

fn userauth(key: &keyringd_ssh_agent_protocol::SshPublicKeyBlob) -> Vec<u8> {
    let mut data = Vec::new();
    write_string(&mut data, b"session");
    data.push(50);
    write_string(&mut data, b"alice");
    write_string(&mut data, b"ssh-connection");
    write_string(&mut data, b"publickey");
    data.push(1);
    write_string(&mut data, key.algorithm().as_bytes());
    write_string(&mut data, &key.to_bytes());
    data
}

keyringd_testkit::scenario! {
    group "Device Key protection integration" {
        case "Protection changes persist the backend key while preserving the Passkey" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "enable and disable operations publish and save the backend result" {
                use crate::service::DeviceKeyProtectionChange;
use crate::model::device_key::LocalAuthentication;
                for replaces_key in [false, true] {
                    let device_service = if replaces_key {
                        SoftwareDeviceKey::generating()
                    } else {
                        SoftwareDeviceKey::new(11, b"fixed-policy-key")
                    };
                    let store = Arc::new(MemoryProfileRepository::default());
                    let application =
                        KeyringdApplication::new(device_service, TestPasskeyService::new(), store.clone())
                            .unwrap();
                    let mut request = ProfileSetupRequest::new("Policy".into(), PasskeyChoice::CreateNew);
                    request.device_key_signature_algorithm = SshSignatureAlgorithm::SkEcdsaSha2NistP256;
                    let original = application
                        .setup_profile(
                            &Default::default(),
                            &Default::default(),
                            request,
                            |_| {},
                            crate::test_support::reject_credential_selection,
                        )
                        .await
                        .unwrap();
                    let id = original.id().clone();
                    let mut changes = application.subscribe_profile_changes();
                    for (operation, expected_authentication) in [
                        (
                            DeviceKeyProtectionChange::Enable,
                            LocalAuthentication::Required,
                        ),
                        (
                            DeviceKeyProtectionChange::Disable,
                            LocalAuthentication::NotRequired,
                        ),
                    ] {
                        application
                            .change_device_key_protection(&Default::default(), &id, operation)
                            .await
                            .unwrap();
                        assert!(changes.has_changed().unwrap());
                        let published = changes
                            .borrow_and_update()
                            .iter()
                            .find(|p| p.id() == &id)
                            .unwrap()
                            .clone();
                        let current = application.profile(&id).unwrap();
                        let saved = store
                            .snapshot()
                            .unwrap()
                            .profiles
                            .into_iter()
                            .find(|p| p.id() == &id)
                            .unwrap();
                        assert_eq!(
                            current.device_key().local_authentication(),
                            expected_authentication
                        );
                        assert_eq!(
                            current.device_key().signature_algorithm(),
                            SshSignatureAlgorithm::SkEcdsaSha2NistP256
                        );
                        assert_eq!(
                            serde_json::to_value(&published).unwrap(),
                            serde_json::to_value(&current).unwrap()
                        );
                        assert_eq!(
                            serde_json::to_value(&saved).unwrap(),
                            serde_json::to_value(&current).unwrap()
                        );
                        assert_eq!(
                            serde_json::to_value(current.passkey()).unwrap(),
                            serde_json::to_value(original.passkey()).unwrap()
                        );
                        assert_eq!(
                            current.device_key().certificate().public_key()
                                != original.device_key().certificate().public_key(),
                            replaces_key
                        );
                    }
                }
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "SSH signature format integration" {
        case "Selected format controls both device identities and survives replacement and restart" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "selected format controls both device identities and survives replacement and restart" {
                for format in [
                    SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                    SshSignatureAlgorithm::EcdsaSha2NistP256,
                    SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256,
                ] {
                    let store = Arc::new(MemoryProfileRepository::default());
                    let device_service = SoftwareDeviceKey::generating();
                    let application = KeyringdApplication::new(
                        device_service.clone(),
                        TestPasskeyService::new(),
                        store.clone(),
                    )
                    .unwrap();
                    let mut request = ProfileSetupRequest::new("Work".into(), PasskeyChoice::CreateNew);
                    request.device_key_signature_algorithm = format.clone();
                    let profile = application
                        .setup_profile(
                            &Default::default(),
                            &Default::default(),
                            request,
                            |_| {},
                            crate::test_support::reject_credential_selection,
                        )
                        .await
                        .unwrap();
                    let id = profile.id().clone();
                    let (key_algorithm, cert_algorithm) = match format {
                        SshSignatureAlgorithm::EcdsaSha2NistP256 => (
                            SshKeyAlgorithm::EcdsaSha2NistP256,
                            SshKeyAlgorithm::EcdsaSha2NistP256Certificate,
                        ),
                        _ => (
                            SshKeyAlgorithm::SkEcdsaSha2NistP256,
                            SshKeyAlgorithm::SkEcdsaSha2NistP256Certificate,
                        ),
                    };
                    assert_eq!(
                        profile
                            .device_key()
                            .certificate()
                            .public_key()
                            .algorithm(),
                        key_algorithm
                    );
                    assert_eq!(
                        profile.device_key().certificate().algorithm(),
                        cert_algorithm
                    );
                    assert_eq!(
                        SshDeviceKeyTrustEntry::from_profile(&profile)
                            .as_str()
                            .starts_with("no-touch-required "),
                        format != SshSignatureAlgorithm::EcdsaSha2NistP256
                    );
                    assert_eq!(
                        SshCertificateAuthorityTrustEntry::from_profile(&profile)
                            .as_str()
                            .contains(",no-touch-required,"),
                        format != SshSignatureAlgorithm::EcdsaSha2NistP256
                    );

                    let handler = KeyringdAgentRequestHandler::new(application.clone());
                    let SshAgentResponse::IdentitiesAnswer { identities } =
                        handler.handle(SshAgentRequest::Identities).await
                    else {
                        panic!("missing identities");
                    };
                    let certificate = SshCertificate::from_bytes(identities[0].key_blob().as_bytes()).unwrap();
                    assert_eq!(certificate.public_key().into_key_blob(), *identities[1].key_blob());
                    assert_eq!(
                        certificate
                            .extensions()
                            .iter()
                            .any(|extension| extension.name() == b"no-touch-required"),
                        format != SshSignatureAlgorithm::EcdsaSha2NistP256
                    );
                    for identity in &identities[..2] {
                        let response = handler
                            .handle(SshAgentRequest::Sign(SshAgentSignRequest::new(
                                identity.key_blob().clone(),
                                userauth(identity.key_blob()),
                                SshAgentSignFlags::default(),
                            )))
                            .await;
                        let SshAgentResponse::Signature { signature } = response else {
                            panic!("missing signature");
                        };
                        assert_eq!(signature.algorithm(), format.clone());
                        if format == SshSignatureAlgorithm::SkEcdsaSha2NistP256 {
                            // The device backend does not claim presence, verification, or a monotonic counter.
                            let SshSignature::SkEcdsa(signature) = signature else {
                                panic!("expected SK signature");
                            };
                            assert_eq!(signature.flags(), 0);
                            assert_eq!(signature.counter(), 0);
                        }
                    }

                    let old_key = ProfileSshIdentity::from_profile_at(
                        &profile,
                        ProfileSshIdentityKind::DeviceKey,
                        std::time::SystemTime::now(),
                    )
                    .unwrap()
                    .to_public_key_line();
                    application.replace_device_key(&Default::default(), &id).await.unwrap();
                    let replacement = application.profiles().remove(0);
                    assert_eq!(replacement.device_key().signature_algorithm(), format);
                    assert_ne!(
                        ProfileSshIdentity::from_profile_at(
                            &replacement,
                            ProfileSshIdentityKind::DeviceKey,
                            std::time::SystemTime::now()
                        )
                        .unwrap()
                        .to_public_key_line(),
                        old_key
                    );
                    let replacement_key = SshDeviceKeyTrustEntry::from_profile(&replacement).to_string();
                    drop(handler);
                    drop(application);
                    let restored =
                        KeyringdApplication::new(device_service, TestPasskeyService::new(), store).unwrap();
                    let restored = restored.profiles().remove(0);
                    assert_eq!(restored.device_key().signature_algorithm(), format);
                    assert_eq!(
                        SshDeviceKeyTrustEntry::from_profile(&restored).to_string(),
                        replacement_key
                    );
                }
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "ProfileSetupRequest::new" {
        case "New profiles default to ecdsa device signatures and security key passkey signatures" {
            verify "new profiles default to ecdsa device signatures and security key passkey signatures" {
                let request = ProfileSetupRequest::new("Work".into(), PasskeyChoice::CreateNew);
                assert_eq!(request.device_key_signature_algorithm, SshSignatureAlgorithm::EcdsaSha2NistP256);
                assert_eq!(
                    request.passkey_signature_algorithm,
                    SshSignatureAlgorithm::SkEcdsaSha2NistP256
                );
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "SSH signature format integration" {
        case "Rejects unsupported signature algorithms before contacting authenticator" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "rejects unsupported signature algorithms before contacting authenticator" {
                use crate::application::ProfileSetupFailure;
                use crate::application::{ApplicationError, OperationRejectionReason};
                use crate::test_support::{MockDeviceKeyService, MockPasskeyService};

                let application = KeyringdApplication::new(
                    MockDeviceKeyService::new(),
                    MockPasskeyService::new(),
                    Arc::new(MemoryProfileRepository::default()),
                )
                .unwrap();
                for choice in [PasskeyChoice::CreateNew, PasskeyChoice::Existing] {
                  for (passkey_algorithm, device_algorithm, reason) in [
                    (SshSignatureAlgorithm::EcdsaSha2NistP256, SshSignatureAlgorithm::EcdsaSha2NistP256, OperationRejectionReason::UnsupportedPasskeySignatureAlgorithm),
                    (SshSignatureAlgorithm::Unsupported("unknown".into()), SshSignatureAlgorithm::EcdsaSha2NistP256, OperationRejectionReason::UnsupportedPasskeySignatureAlgorithm),
                    (SshSignatureAlgorithm::SkEcdsaSha2NistP256, SshSignatureAlgorithm::Unsupported("unknown".into()), OperationRejectionReason::UnsupportedDeviceKeySignatureAlgorithm),
                  ] {
                    let mut request = ProfileSetupRequest::new("Work".into(), choice);
                    request.passkey_signature_algorithm = passkey_algorithm;
                    request.device_key_signature_algorithm = device_algorithm;
                    assert!(matches!(
                        application
                            .setup_profile(
                                &Default::default(),
                                &Default::default(),
                                request,
                                |_| {},
                                crate::test_support::reject_credential_selection,
                            )
                            .await,
                        Err(ProfileSetupFailure::Failed(ApplicationError::OperationRejected(
                                rejection
                            ))) if rejection == reason
                    ));
                  }
                }
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "SSH signature format integration" {
        case "Webauthn passkey format signs certificates and direct authentication" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "webauthn passkey format signs certificates and direct authentication" {
                let application = KeyringdApplication::new(
                    SoftwareDeviceKey::generating(),
                    TestPasskeyService::new(),
                    Arc::new(MemoryProfileRepository::default()),
                )
                .unwrap();
                let mut request = ProfileSetupRequest::new("WebAuthn passkey".into(), PasskeyChoice::CreateNew);
                request.passkey_signature_algorithm = SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256;
                let original = application
                    .setup_profile(
                        &Default::default(),
                        &Default::default(),
                        request,
                        |_| {},
                        crate::test_support::reject_credential_selection,
                    )
                    .await
                    .unwrap();
                let mut recovery =
                    ProfileSetupRequest::new("Recovered WebAuthn passkey".into(), PasskeyChoice::Existing);
                recovery.passkey_signature_algorithm = original.passkey().signature_algorithm();
                let recovered = application
                    .setup_profile(
                        &Default::default(),
                        &Default::default(),
                        recovery,
                        |_| {},
                        crate::test_support::reject_credential_selection,
                    )
                    .await
                    .unwrap();
                assert_eq!(
                    recovered.passkey().public_key(),
                    original.passkey().public_key()
                );
                let handler = KeyringdAgentRequestHandler::new(application);
                let SshAgentResponse::IdentitiesAnswer { identities } =
                    handler.handle(SshAgentRequest::Identities).await
                else {
                    panic!("missing identities");
                };
                let certificate = SshCertificate::from_bytes(identities[0].key_blob().as_bytes()).unwrap();
                assert_eq!(
                    certificate.signature().algorithm(),
                    SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256
                );
                let passkey = original.passkey().to_ssh_public_key().into_key_blob();
                let SshAgentResponse::Signature { signature } = handler
                    .handle(SshAgentRequest::Sign(SshAgentSignRequest::new(
                        passkey.clone(),
                        userauth(&passkey),
                        SshAgentSignFlags::default(),
                    )))
                    .await
                else {
                    panic!("missing Passkey signature");
                };
                assert_eq!(
                    signature.algorithm(),
                    SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256
                );
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "Profile signing" {
        case "Rejects stale Device Key and certificate requests after key replacement" {
            why "A profile's Device Key can change between SSH identity selection and signing."
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "signing rechecks the requested identity against the current profile" {
                use crate::application::{ApplicationError, OperationRejectionReason};
                let (application, _, _, id) = super::fixture::completed_application().await;
                let context = Default::default();
                let old = application.profile(&id).unwrap();
                let requests = [
                    old.device_key().certificate().public_key().into_key_blob(),
                    old.device_key().certificate().to_key_blob(),
                ].map(|key| SshAgentSignRequest::new(key.clone(), userauth(&key), Default::default()));
                application.replace_device_key(&context, &id).await.unwrap();
                for request in requests {
                    assert!(matches!(
                        application.sign_with_device_key(&context, &id, &request).await,
                        Err(ApplicationError::OperationRejected(OperationRejectionReason::SshIdentityUnavailable))
                    ));
                }
                let current = application.profile(&id).unwrap();
                let key = current.device_key().certificate().public_key().into_key_blob();
                let request = SshAgentSignRequest::new(key.clone(), userauth(&key), Default::default());
                application.sign_with_device_key(&context, &id, &request).await.unwrap();
            }
        }
        case "Both signing paths reject mismatched identities disabled profiles and removed profiles" {
            why "Profile state and the requested key must be checked by the signing API rather than relying on a caller's earlier snapshot."
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "each path accepts its current identity and rejects invalid current profile state" {
                use crate::application::{ApplicationError, OperationRejectionReason};
                let (application, _, _, id) = super::fixture::completed_application().await;
                let context = Default::default();
                let profile = application.profile(&id).unwrap();
                let device_key = profile.device_key().certificate().public_key().into_key_blob();
                let passkey = profile.passkey().to_ssh_public_key().into_key_blob();
                let device_request = SshAgentSignRequest::new(device_key.clone(), userauth(&device_key), Default::default());
                let passkey_request = SshAgentSignRequest::new(passkey.clone(), userauth(&passkey), Default::default());
                for result in [
                    application.sign_with_device_key(&context, &id, &passkey_request).await,
                    application.sign_with_passkey(&id, &device_request).await,
                ] {
                    assert!(matches!(result, Err(ApplicationError::OperationRejected(OperationRejectionReason::SshIdentityUnavailable))));
                }
                application.sign_with_device_key(&context, &id, &device_request).await.unwrap();
                application.sign_with_passkey(&id, &passkey_request).await.unwrap();
                application.set_profile_enabled(&id, false).unwrap();
                for result in [
                    application.sign_with_device_key(&context, &id, &device_request).await,
                    application.sign_with_passkey(&id, &passkey_request).await,
                ] {
                    assert!(matches!(result, Err(ApplicationError::OperationRejected(OperationRejectionReason::ProfileDisabled))));
                }
                application.remove_profile(&context, &id).await.unwrap();
                for result in [
                    application.sign_with_device_key(&context, &id, &device_request).await,
                    application.sign_with_passkey(&id, &passkey_request).await,
                ] {
                    assert!(matches!(result, Err(ApplicationError::NotFound("profile"))));
                }
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "Device Key application preservation" {
        case "Preserves the certificate application through protection changes and key replacement" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "unchanged keys retain their certificate without Passkey access and regenerated certificates retain the application" {
                use crate::model::device_key::DeviceKey;
use crate::service::DeviceKeyProtectionChange;
                use crate::persistence::ProfilesDocument;
                use crate::signing::fido::PasskeyAssertionProvider;
                use crate::test_support::TestFidoMode;
                use keyringd_ssh_agent_protocol::{SshCertificateBody, SshPublicKey};
                for algorithm in [SshSignatureAlgorithm::SkEcdsaSha2NistP256, SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256] {
                    for replaces_key in [false, true] {
                        let service = if replaces_key { SoftwareDeviceKey::generating() } else { SoftwareDeviceKey::new(11, b"fixed-key") };
                        let fido = Arc::new(TestPasskeyService::new());
                        let initial = KeyringdApplication::new(service.clone(), fido.clone(), Arc::new(MemoryProfileRepository::default())).unwrap();
                        let mut request = ProfileSetupRequest::new("Application".into(), PasskeyChoice::CreateNew);
                        request.device_key_signature_algorithm = algorithm.clone();
                        let mut profile = initial.setup_profile(&Default::default(), &Default::default(), request, |_| {}, crate::test_support::reject_credential_selection).await.unwrap();
                        let original = profile.device_key();
                        let mut body = original.certificate().body().clone();
                        let SshCertificateBody::SkEcdsaP256(subject) = &mut body else { unreachable!() };
                        subject.application = b"ssh:other".to_vec();
                        let prepared = body.prepare().unwrap();
                        let mut target = crate::execution::operation::FidoOperationTarget::Unselected;
                        let signature = PasskeyAssertionProvider::new(fido.as_ref(), &mut target)
                            .sign_ssh(profile.passkey(), prepared.signing_bytes())
                            .await
                            .unwrap();
                        let certificate = prepared.with_signature(signature);
                        let key = DeviceKey::new(original.handle().clone(), certificate.clone(), original.local_authentication(), algorithm.clone()).unwrap();
                        profile.replace_device_key(key);
                        let id = profile.id().clone();
                        let store = Arc::new(MemoryProfileRepository::with_state(ProfilesDocument { profiles: vec![profile], ..Default::default() }));
                        let app = KeyringdApplication::new(service, fido.clone(), store.clone()).unwrap();
                        if !replaces_key { fido.set_mode(TestFidoMode::Failure); }
                        app.change_device_key_protection(&Default::default(), &id, DeviceKeyProtectionChange::Enable).await.unwrap();
                        let updated = app.profile(&id).unwrap();
                        assert_eq!(updated.device_key().certificate().public_key() != certificate.public_key(), replaces_key);
                        if !replaces_key { assert_eq!(updated.device_key().certificate().to_bytes(), certificate.to_bytes()); }
                        assert_eq!(updated.device_key().relying_party_id(), Some("ssh:other"));
                        fido.set_mode(TestFidoMode::Success);
                        app.replace_device_key(&Default::default(), &id).await.unwrap();
                        let updated = app.profile(&id).unwrap();
                        let SshPublicKey::SkEcdsaP256(key) = updated.device_key().certificate().public_key() else { unreachable!() };
                        assert_eq!(key.application(), b"ssh:other");
                        assert_eq!(updated.device_key().signature_algorithm(), algorithm);
                        let saved = ProfilesDocument::from_json(&store.snapshot().unwrap().to_json().unwrap()).unwrap();
                        assert_eq!(saved.profiles[0].device_key().certificate().to_bytes(), updated.device_key().certificate().to_bytes());
                    }
                }
            }
        }
    }
}
