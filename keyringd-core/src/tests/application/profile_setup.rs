use super::fixture::*;
use crate::application::ProfileSetupFailure;
use crate::application::ProfileSetupProgress;
use crate::model::passkey::CredentialId;
use crate::test_support::cancelled_fido;
use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;

keyringd_testkit::scenario! {
    group "KeyringdApplication::setup_profile" {
        case "Rejects mismatched registration client data before provisioning or publishing a profile" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            given "a registration backend that returns client data for another challenge" {
                use crate::test_support::MockPasskeyService;
                use keyringd_fido::CollectedClientData;
                use keyringd_fido_client::ClientData;

                let mut fido = MockPasskeyService::new();
                fido.expect_make_credential().times(1).return_once(|options, provider| {
                    let stale_data = ClientData::try_from(
                        CollectedClientData::create(&[7; 32], Passkey::WEB_AUTHN_ORIGIN).unwrap(),
                    ).unwrap();
                    assert_ne!(provider.provide(options.challenge()).unwrap(), stale_data);
                    Box::pin(async move {
                        TestPasskeyService::new().make_credential(
                            options,
                            Arc::new(move |_: &[u8]| Ok(stale_data.clone())),
                        ).await
                    })
                });
                let mut keys = MockDeviceKeyService::new();
                keys.expect_generate_key().never();
                let store = Arc::new(MemoryRepository::default());
                let application = KeyringdApplication::new(keys, fido, store.clone()).unwrap();
            }
            when "a new profile is requested" {
                let result = application.setup_profile(
                    &Default::default(),
                    &Default::default(),
                    ProfileSetupRequest::new("Test".into(), PasskeyChoice::CreateNew),
                    |_| {},
                    crate::test_support::reject_credential_selection,
                ).await;
            }
            then "setup rejects the authenticator response without provisioning or publishing a profile" {
                assert!(matches!(result, Err(ProfileSetupFailure::Failed(ApplicationError::InvalidAuthenticatorResponse(_)))));
                assert!(application.profiles().is_empty());
                assert!(store.snapshot().unwrap().profiles.is_empty());
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "KeyringdApplication::setup_profile" {
        case "Preserves setup failure when device key cleanup also fails" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "preserves setup failure when device key cleanup also fails" {
                use crate::service::DeviceKeyMaterial;
                use tracing::instrument::WithSubscriber;
                use crate::test_support::{MockDeviceKeyServiceError, TestFidoMode, test_device_public_key};

                for operation_error in [
                    ApplicationError::PasskeyCredentialNotFound,
                    ApplicationError::ProfileRepository(std::io::Error::other("save failed")),
                    ApplicationError::OperationCanceled,
                ] {
                    let cancel_after_generation = matches!(operation_error, ApplicationError::OperationCanceled);
                    let cancellation = tokio_util::sync::CancellationToken::new();
                    let fail_certificate =
                        matches!(operation_error, ApplicationError::PasskeyCredentialNotFound);
                    let generation_cancellation = cancellation.clone();
                    let mut device_keys = MockDeviceKeyService::new();
                    device_keys
                        .expect_generate_key()
                        .once()
                        .returning(move |_, request| {
                            if cancel_after_generation {
                                generation_cancellation.cancel();
                            }
                            Ok(DeviceKeyMaterial {
                                handle: DevicePrivateKeyHandle::new(b"uncommitted-key".to_vec()),
                                public_key: test_device_public_key(request.algorithm),
                            })
                        });
                    device_keys
                        .expect_delete_key()
                        .withf(|_, handle| handle.as_ref() == b"uncommitted-key")
                        .once()
                        .returning(|_, _| Err(MockDeviceKeyServiceError));
                    let store = Arc::new(MemoryRepository::default());
                    store.set_save_failure(matches!(operation_error, ApplicationError::ProfileRepository(_)));
                    let fido = Arc::new(TestPasskeyService::new());
                    let application =
                        KeyringdApplication::new(device_keys, fido.clone(), store.clone()).unwrap();

                    let log = tempfile::NamedTempFile::new().unwrap();
                    let subscriber = tracing_subscriber::fmt()
                        .without_time()
                        .with_ansi(false)
                        .with_target(false)
                        .with_writer(std::sync::Mutex::new(log.reopen().unwrap()))
                        .finish();
                    let failure = application
                        .setup_profile(
                            &Default::default(),
                            &cancellation,
                            ProfileSetupRequest::new("Work".to_owned(), PasskeyChoice::CreateNew),
                            |step| {
                                if fail_certificate && step == ProfileSetupProgress::ProvisioningDeviceKey {
                                    fido.set_mode(TestFidoMode::NoCredentials);
                                }
                            },
                            crate::test_support::reject_credential_selection,
                        )
                        .with_subscriber(subscriber)
                        .await
                        .err()
                        .expect("profile setup must fail");

                    let output = std::fs::read_to_string(log.path()).unwrap();
                    let cleanup_logs = output.lines().filter(|line| line.contains("unused device key could not be deleted")).collect::<Vec<_>>();
                    assert_eq!(cleanup_logs.len(), 1, "{output}");
                    let cleanup_log = cleanup_logs[0];
                    for expected in ["WARN", "exec_id=", "setup_profile", "mock device key store error", "MockDeviceKeyServiceError"] {
                        assert!(cleanup_log.contains(expected), "missing {expected}: {cleanup_log}");
                    }
                    match operation_error {
                        ApplicationError::PasskeyCredentialNotFound => assert!(matches!(
                            failure,
                            ProfileSetupFailure::PasskeyNotFound
                        )),
                        ApplicationError::OperationCanceled => {
                            assert!(matches!(failure, ProfileSetupFailure::Cancelled))
                        }
                        ApplicationError::ProfileRepository(_) => assert!(matches!(
                            failure,
                            ProfileSetupFailure::Failed(ApplicationError::ProfileRepository(_))
                        )),
                        _ => unreachable!(),
                    }
                    assert!(application.profiles().is_empty());
                    assert!(store.snapshot().unwrap().profiles.is_empty());
                }
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "KeyringdApplication::setup_profile" {
        case "Completes every profile-setup stage in order" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "an empty application and a progress recorder" {
                let persistence = Arc::new(MemoryRepository::default());
                let fido = Arc::new(TestPasskeyService::new());
                let application = KeyringdApplication::new(
                    crate::test_support::successful_device_keys(),
                    Arc::clone(&fido),
                    persistence.clone(),

                )
                .unwrap();
                let mut progress = Vec::new();
            }
            when "a profile is set up with a newly created Passkey" {
                let profile = application
                    .setup_profile(
                        &Default::default(),
                        &Default::default(),
                        ProfileSetupRequest::new("Work".to_owned(), PasskeyChoice::CreateNew),
                        |step| {
                            progress.push(step);
                        },
                        crate::test_support::reject_credential_selection,
                    )
                    .await
                    .unwrap();
            }
            then "setup returns and publishes the completed profile" {
                assert_eq!(profile.display_name(), "Work");
                assert_eq!(application.profiles().len(), 1);
                assert_eq!(persistence.snapshot().unwrap().profiles.len(), 1);
            }
            then "setup reports every stage in execution order" {
                assert_eq!(
                    progress,
                    [
                        ProfileSetupProgress::Starting,
                        ProfileSetupProgress::CreatingPasskey,
                        ProfileSetupProgress::ProvisioningDeviceKey,
                        ProfileSetupProgress::CommittingProfile,
                    ]
                );
            }
            then "setup carries the successful transport into the next FIDO operation" {
                assert_eq!(fido.requested_hints(), [vec![], vec![PublicKeyCredentialHint::security_key()]]);
            }
        }

        case "Reports that no existing Passkey can be found" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "an empty application with no discoverable Passkey credential" {
                let application = KeyringdApplication::new(
                    crate::test_support::successful_device_keys(),
                    TestPasskeyService::new(),
                    Arc::new(MemoryRepository::default()),

                )
                .unwrap();
                let mut progress = Vec::new();
            }
            when "profile setup requests an existing Passkey" {
                let result = application
                    .setup_profile(
                        &Default::default(),
                        &Default::default(),
                        ProfileSetupRequest::new("Work".to_owned(), PasskeyChoice::Existing),
                        |step| {
                            progress.push(step);
                        },
                        crate::test_support::reject_credential_selection,
                    )
                    .await;
            }
            then "setup reports PasskeyNotFound without publishing a profile" {
                assert!(matches!(
                    result,
                    Err(ProfileSetupFailure::PasskeyNotFound)
                ));
                assert!(application.profiles().is_empty());
            }
            then "progress stops after attempting Passkey discovery" {
                assert_eq!(
                    progress,
                    [
                        ProfileSetupProgress::Starting,
                        ProfileSetupProgress::DiscoveringPasskey,
                    ]
                );
            }
        }

        case "Stops before Device Key provisioning when cancellation is requested" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "an empty application that can create a Passkey" {
                let device_keys = TrackingDeviceKeyService::default();
                let application = KeyringdApplication::new(
                    device_keys.clone(),
                    TestPasskeyService::new(),
                    Arc::new(MemoryRepository::default()),

                )
                .unwrap();
            }
            when "cancellation is requested before Device Key provisioning" {
                let cancellation = tokio_util::sync::CancellationToken::new();
                let result = application
                    .setup_profile(
                        &Default::default(),
                        &cancellation,
                        ProfileSetupRequest::new("Work".to_owned(), PasskeyChoice::CreateNew),
                        |step| {
                            if step == ProfileSetupProgress::ProvisioningDeviceKey {
                                cancellation.cancel();
                            }
                        },
                        crate::test_support::reject_credential_selection,
                    )
                    .await;
            }
            then "setup reports cancellation without generating or publishing a Device Key" {
                assert!(matches!(
                    result,
                    Err(ProfileSetupFailure::Cancelled)
                ));
                assert_eq!(device_keys.generated_count(), 0);
                assert!(application.profiles().is_empty());
            }
        }

        case "Deletes the provisioned Device Key when cancellation occurs before commit" {
            why "An unsuccessful setup must clean up its provisioned private key before returning."
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "an empty application with a Device Key service that records deletion" {
                let device_keys = TrackingDeviceKeyService::default();
                let application = KeyringdApplication::new(
                    device_keys.clone(),
                    TestPasskeyService::new(),
                    Arc::new(MemoryRepository::default()),

                )
                .unwrap();
            }
            when "cancellation is requested after provisioning and before profile commit" {
                let cancellation = tokio_util::sync::CancellationToken::new();
                let result = application
                    .setup_profile(
                        &Default::default(),
                        &cancellation,
                        ProfileSetupRequest::new("Work".to_owned(), PasskeyChoice::CreateNew),
                        |step| {
                            if step == ProfileSetupProgress::CommittingProfile {
                                cancellation.cancel();
                            }
                        },
                        crate::test_support::reject_credential_selection,
                    )
                    .await;
            }
            then "setup reports cancellation without publishing a profile" {
                assert!(matches!(
                    result,
                    Err(ProfileSetupFailure::Cancelled)
                ));
                assert!(application.profiles().is_empty());
            }
            then "the provisioned Device Key is deleted as compensation" {
                assert_eq!(device_keys.generated_count(), 1);
                assert_eq!(device_keys.deleted_handles().len(), 1);
                assert_eq!(device_keys.deleted_handles()[0].as_ref(), b"tracking-key-1");
            }
        }

        case "Recovers an existing Passkey before provisioning the profile" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "a FIDO service containing a discoverable Passkey credential" {
                use crate::test_support::MockPasskeyService;
                use std::sync::Mutex;

                let fido = Arc::new(TestPasskeyService::new());
                let (options, parameters) = passkey_make_options(b"existing-passkey");
                let expected_user_handle = options.user().id().to_vec();
                let registered = fido.make_credential(options, parameters).await.unwrap();
                let authenticator_data = registered.credential().response().authenticator_data().unwrap();
                let attested = authenticator_data.attested_credential_data().unwrap();
                let expected_public_key = attested.credential_public_key().es256_sec1_public_key().unwrap();
                let expected_credential_id = attested.credential_id().to_vec();
                let requests = Arc::new(Mutex::new(Vec::new()));
                let observed = requests.clone();
                let mut service = MockPasskeyService::new();
                service.expect_get_assertions().times(3).returning(move |options, device_id, provider| {
                    observed.lock().unwrap().push((
                        options.challenge().to_vec(),
                        options.allow_credentials().iter().map(|credential| credential.id().to_vec()).collect::<Vec<_>>(),
                    ));
                    let fido = fido.clone();
                    Box::pin(async move { fido.get_assertions(options, device_id, provider).await })
                });
                let application = KeyringdApplication::new(
                    crate::test_support::successful_device_keys(),
                    service,
                    Arc::new(MemoryRepository::default()),

                )
                .unwrap();
                let mut progress = Vec::new();
            }
            when "profile setup selects the existing Passkey" {
                let profile = application
                    .setup_profile(
                        &Default::default(),
                        &Default::default(),
                        ProfileSetupRequest::new("Recovered".to_owned(), PasskeyChoice::Existing),
                        |step| {
                            progress.push(step);
                        },
                        crate::test_support::reject_credential_selection,
                    )
                    .await
                    .unwrap();
            }
            then "setup verifies the recovered key and publishes the profile" {
                assert_eq!(profile.passkey().credential_id().as_bytes(), expected_credential_id);
                assert_eq!(profile.passkey().public_key(), expected_public_key);
                assert_eq!(profile.passkey().user_handle(), expected_user_handle);
                assert_eq!(profile.display_name(), "Recovered");
                assert_eq!(application.profiles().len(), 1);
                assert!(progress.contains(&ProfileSetupProgress::DiscoveringPasskey));
                assert!(progress.contains(&ProfileSetupProgress::RecoveringPasskeyPublicKey));
            }
            then "key recovery requests a different challenge from the discovered credential" {
                let requests = requests.lock().unwrap();
                assert!(requests[0].1.is_empty());
                assert_ne!(requests[0].0, requests[1].0);
                assert_eq!(requests[1].1, vec![expected_credential_id]);
            }
        }

        case "Requires a choice among multiple Passkeys and pins the selected credential" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "two discoverable credentials on the same authenticator" {
                let fido = Arc::new(TestPasskeyService::new());
                let (options, parameters) = passkey_make_options(b"first-passkey");
                let first = fido.make_credential(options,
parameters).await.unwrap();
                let (options, parameters) = passkey_make_options(b"second-passkey");
                let second = fido.make_credential(options,
parameters).await.unwrap();
                let selected_id = second.credential().raw_id().to_vec();
                let unselected_id = first.credential().raw_id().to_vec();
                let persistence = Arc::new(MemoryRepository::default());
                let application = KeyringdApplication::new(
                    crate::test_support::successful_device_keys(),
                    Arc::clone(&fido),
                    persistence.clone(),

                )
                .unwrap();
                let mut offered = Vec::new();
            }
            when "the user chooses the second credential" {
                let result = application
                    .setup_profile(
                        &Default::default(),
                        &Default::default(),
                        ProfileSetupRequest::new("Recovered".to_owned(), PasskeyChoice::Existing),
                        |_| {},
                        |candidates| {
                            offered = candidates;
                            std::future::ready(Ok(CredentialId::new(selected_id.clone())))
                        },
                    )
                    .await;
            }
            then "both candidates are distinguishable and the chosen ID is persisted" {
                assert!(result.is_ok());
                assert_eq!(offered.len(), 2);
                assert_ne!(offered[0].label(), offered[1].label());
                assert!(offered.iter().any(|candidate| candidate.credential_id() == &CredentialId::new(unselected_id.clone())));
                assert!(offered.iter().any(|candidate| candidate.credential_id() == &CredentialId::new(selected_id.clone())));
                assert_eq!(
                    persistence.snapshot().unwrap().profiles[0].passkey().credential_id().as_bytes(),
                    selected_id
                );
                let requests = fido.requested_credential_ids();
                assert!(requests[0].is_empty(), "discovery must not constrain credentials");
                assert_eq!(requests[1], vec![selected_id.clone()]);
                assert_eq!(requests[2], vec![selected_id.clone()]);
            }
        }

        case "Cancelling a choice among multiple Passkeys leaves no profile" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "two discoverable Passkeys" {
                let fido = Arc::new(TestPasskeyService::new());
                for name in [b"first-passkey".as_slice(), b"second-passkey".as_slice()] {
                    let (options, parameters) = passkey_make_options(name);
                    fido.make_credential(options,
parameters).await.unwrap();
                }
                let application = KeyringdApplication::new(
                    crate::test_support::successful_device_keys(),
                    fido,
                    Arc::new(MemoryRepository::default()),

                ).unwrap();
            }
            when "the user cancels the credential picker" {
                let result = application.setup_profile(
                    &Default::default(),
                    &Default::default(),
                    ProfileSetupRequest::new("Cancelled".to_owned(), PasskeyChoice::Existing),
                    |_| {},
                    |_| std::future::ready(Err(crate::application::ApplicationError::OperationCanceled)),
                ).await;
            }
            then "setup is cancelled without creating a profile" {
                assert!(matches!(result, Err(ProfileSetupFailure::Cancelled)));
                assert!(application.profiles().is_empty());
            }
        }

        case "Allows concurrent profiles to use the same display name" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "an empty application" {
                let application = KeyringdApplication::new(
                    crate::test_support::successful_device_keys(),
                    TestPasskeyService::new(),
                    Arc::new(MemoryRepository::default()),

                )
                .unwrap();
            }
            when "two concurrent setup requests use exactly the same display name" {
                let cancellation = tokio_util::sync::CancellationToken::new();
                let (first, second) = tokio::join!(
                    application.setup_profile(
                        &(),
                        &cancellation,
                        ProfileSetupRequest::new("Work".to_owned(), PasskeyChoice::CreateNew),
                        |_| {},
                        crate::test_support::reject_credential_selection,
                    ),
                    application.setup_profile(
                        &(),
                        &cancellation,
                        ProfileSetupRequest::new("Work".to_owned(), PasskeyChoice::CreateNew),
                        |_| {},
                        crate::test_support::reject_credential_selection,
                    ),
                );
            }
            then "both setup requests complete and publish separate profiles" {
                let first = first.unwrap();
                let second = second.unwrap();
                assert_eq!(first.display_name(), "Work");
                assert_eq!(second.display_name(), "Work");
                assert_ne!(first.id(), second.id());
                assert_eq!(application.profiles().len(), 2);
            }
        }

        case "Reports user cancellation while creating a Passkey" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);

            given "an application whose FIDO operation is canceled by the user" {
                let application = KeyringdApplication::new(
                    crate::test_support::successful_device_keys(),
                    cancelled_fido(),
                    Arc::new(MemoryRepository::default()),

                )
                .unwrap();
            }
            when "profile setup attempts to create the Passkey" {
                let result = application
                    .setup_profile(
                        &Default::default(),
                        &Default::default(),
                        ProfileSetupRequest::new("Work".to_owned(), PasskeyChoice::CreateNew),
                        |_| {},
                        crate::test_support::reject_credential_selection,
                    )
                    .await;
            }
            then "setup reports cancellation without publishing a profile" {
                assert!(matches!(
                    result,
                    Err(ProfileSetupFailure::Cancelled)
                ));
                assert!(application.profiles().is_empty());
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "Profile setup" {
        case "Profile setup issues a Device Certificate for the generated Device Key with the profile Passkey as CA" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "the certificate contains the Device Key and names the profile Passkey as CA" {
                use keyringd_ssh_agent_protocol::{SshSignature, };

                use crate::model::device_key::DeviceKeyAlgorithm;
                use crate::test_support::{successful_device_keys, test_device_public_key};
                let expected_key = test_device_public_key(DeviceKeyAlgorithm::EcdsaP256)
                    .to_ssh_public_key(None).unwrap();
                let app = crate::application::KeyringdApplication::new(
                    successful_device_keys(),
                    crate::test_support::TestPasskeyService::default(),
                    Arc::new(crate::test_support::MemoryProfileRepository::default()),
                ).unwrap();
                let profile = app.setup_profile(
                    &Default::default(),
                    &Default::default(),
                    ProfileSetupRequest::new("Certificate test".into(), PasskeyChoice::CreateNew),
                    |_| {},
                    crate::test_support::reject_credential_selection,
                ).await.unwrap();
                let certificate = profile.device_key().certificate();
                assert_eq!(certificate.public_key(), expected_key);
                assert_eq!(
                    certificate.signature_key(),
                    &profile.passkey().to_ssh_public_key()
                );
                assert_eq!(
                    certificate.signature().algorithm(),
                    SshSignatureAlgorithm::SkEcdsaSha2NistP256
                );
                assert!(matches!(certificate.signature(), SshSignature::SkEcdsa(_)));
            }
        }
    }
}

#[tokio::test]
async fn large_blob_attestation_skips_the_second_recovery_assertion() {
    use crate::test_support::successful_device_keys;
    for (supported, corrupt, expected_requests) in
        [(true, false, 2), (true, true, 3), (false, false, 3)]
    {
        let fido = if supported {
            TestPasskeyService::new().with_large_blob_support()
        } else {
            TestPasskeyService::new()
        };
        let fido = Arc::new(fido);
        let application = KeyringdApplication::new(
            successful_device_keys(),
            fido.clone(),
            Arc::new(MemoryRepository::default()),
        )
        .unwrap();
        let original = application
            .setup_profile(
                &Default::default(),
                &Default::default(),
                ProfileSetupRequest::new("Original".into(), PasskeyChoice::CreateNew),
                |_| {},
                crate::test_support::reject_credential_selection,
            )
            .await
            .unwrap();
        assert!(original.passkey().attestation_response().is_some());
        if corrupt {
            fido.set_large_blob(
                original.passkey().credential_id().as_bytes().to_vec(),
                b"invalid registration data".to_vec(),
            );
        }
        let before = fido.requests();
        let mut recovery_fido = crate::test_support::MockPasskeyService::new();
        let authenticator = fido.clone();
        recovery_fido
            .expect_get_assertions()
            .returning(move |options, device_id, provider| {
                assert!(
                    options
                        .extensions()
                        .and_then(|e| e.large_blob())
                        .and_then(|input| input.write())
                        .is_none(),
                    "recovery must not request a largeBlob write"
                );
                let authenticator = authenticator.clone();
                Box::pin(async move {
                    authenticator
                        .get_assertions(options, device_id, provider)
                        .await
                })
            });
        let recovery_application = KeyringdApplication::new(
            successful_device_keys(),
            recovery_fido,
            Arc::new(MemoryRepository::default()),
        )
        .unwrap();
        let recovered = recovery_application
            .setup_profile(
                &Default::default(),
                &Default::default(),
                ProfileSetupRequest::new("Recovered".into(), PasskeyChoice::Existing),
                |_| {},
                crate::test_support::reject_credential_selection,
            )
            .await
            .unwrap();
        assert_eq!(
            fido.requests() - before,
            expected_requests,
            "supported={supported}, corrupt={corrupt}"
        );
        assert_eq!(
            recovered.passkey().public_key(),
            original.passkey().public_key()
        );
        assert_eq!(
            recovered.passkey().user_handle(),
            original.passkey().user_handle()
        );
        assert_eq!(
            recovered.passkey().attestation_response().is_some(),
            supported && !corrupt
        );
        if supported && !corrupt {
            assert_eq!(
                recovered.passkey().attestation_response(),
                original.passkey().attestation_response()
            );
        }
        let encoded = serde_json::to_vec(&recovered).unwrap();
        let restored: Profile = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(
            restored.passkey().attestation_response(),
            recovered.passkey().attestation_response()
        );
    }
}
