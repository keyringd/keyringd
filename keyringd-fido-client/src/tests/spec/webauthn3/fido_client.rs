use crate::ClientData;
use crate::client::FidoClient;
use crate::client::FidoClientError;
use crate::client::PreparedAssertion;
use crate::client::PreparedMakeCredential;
use crate::client::test_support::LifecycleCalls;
use crate::client::test_support::OsBackendCalls;
use crate::client::test_support::lifecycle_handler;
use crate::client::test_support::pending_os_backend;
use crate::test_support::assert_fido_error;
use crate::test_support::empty_usb_backend;
use crate::test_support::usb_only_handler;
use crate::tests::support::client_fido_client::os_test_client;
use crate::tests::support::client_fido_client::wait_for_active_backend;
use keyringd_fido::webauthn::AttestationConveyancePreference;
use keyringd_fido::webauthn::AuthenticatorAttachment;
use keyringd_fido::webauthn::AuthenticatorTransport;
use keyringd_fido::webauthn::PublicKeyCredentialCreationOptions;
use keyringd_fido::webauthn::PublicKeyCredentialDescriptor;
use keyringd_fido::webauthn::PublicKeyCredentialParameters;
use keyringd_fido::webauthn::PublicKeyCredentialRequestOptions;
use keyringd_fido::webauthn::PublicKeyCredentialRpEntity;
use keyringd_fido::webauthn::PublicKeyCredentialType;
use keyringd_fido::webauthn::PublicKeyCredentialUserEntity;
use keyringd_fido::webauthn::ResidentKeyRequirement;
use keyringd_fido::webauthn::UserVerificationRequirement;
use std::sync::Arc;

keyringd_testkit::scenario! {
        case "Allow only cross-platform candidates that match the Authenticator Attachment" {
            spec "https://www.w3.org/TR/webauthn-3/#dom-authenticatorselectioncriteria-authenticatorattachment"

            given "Prepare unspecified platforms, cross-platform, and unknown attachment values" {
                let platform = AuthenticatorAttachment::Platform;
                let cross_platform = AuthenticatorAttachment::CrossPlatform;
                let unknown = AuthenticatorAttachment::Unknown("future-attachment".to_owned());
                let attachments = [
                    None,
                    Some(&platform),
                    Some(&cross_platform),
                    Some(&unknown),
                ];
            }
            when "Determine the Eligibility of the currently implemented Cross-Platform Authenticator" {
                let eligible = attachments.map(PreparedMakeCredential::allows_cross_platform_authenticator);
            }
            then "Excluding only the platform and treating unspecified and unknown values in the same way" {
                assert_eq!(eligible, [true, false, true, true]);
            }
        }
        case "Do not start Cross-Platform Transport when PLATFORM ATTACHMENT is specified" {
            spec "https://www.w3.org/TR/webauthn-3/#dom-authenticatorselectioncriteria-authenticatorattachment"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare a Credential Creation Option to request a Platform Attachment" {
                let client = FidoClient::new(usb_only_handler()).with_usb_backend(crate::backend::UsbBackend::Hid(std::sync::Arc::new(empty_usb_backend()))).with_hybrid_connector(crate::backend::SystemHybridChannelConnector::default());
                let options = PublicKeyCredentialCreationOptions::new(
                    PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                    PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                    vec![2],
                    vec![PublicKeyCredentialParameters::es256()],
                )
                .with_authenticator_selection(
                    keyringd_fido::webauthn::AuthenticatorSelectionCriteria::default()
                        .with_authenticator_attachment(AuthenticatorAttachment::Platform)
                        .with_resident_key(ResidentKeyRequirement::Required),
                );
                let client_data_requested = Arc::new(std::sync::atomic::AtomicBool::new(false));
                let requested = Arc::clone(&client_data_requested);
                let parameters = std::sync::Arc::new(move |_: &[u8]| {
                    requested.store(true, std::sync::atomic::Ordering::Relaxed);
                    Ok(ClientData::from_bytes(vec![3]))
                });
            }
            when "Start Credential Creation" {
                let result = client.make_credential(options, keyringd_fido_client::MakeCredentialContext::new((parameters).as_ref())).await;
            }
            then "Return NoAuthenticator before transport starts because there are no platform candidates available" {
                assert_fido_error!(result, FidoClientError::NoAuthenticator);
                assert!(!client_data_requested.load(std::sync::atomic::Ordering::Relaxed));
            }
        }
        case "OS USB Exposes Operation Lifecycle and Cancel ID" {
            spec "https://www.w3.org/TR/webauthn-3/#sctn-abortoperation"
            spec "https://www.w3.org/TR/webauthn-3/#sctn-op-cancel"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare the OS USB Backend and Client to Record Operations" {
                let handler = Arc::new(LifecycleCalls::default());
                let backend = Arc::new(OsBackendCalls::default());
                let backend_contract = pending_os_backend(backend.clone());
                let client = Arc::new(os_test_client(lifecycle_handler(handler.clone()), backend_contract));
                let options = PublicKeyCredentialCreationOptions::new(
                    PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                    PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                    vec![2],
                    vec![PublicKeyCredentialParameters::new(
                        PublicKeyCredentialType::PublicKey,
                        keyringd_fido::webauthn::CoseAlgorithmIdentifier::ES256,
                    )],
                )
                .with_timeout(10_000)
                .with_authenticator_selection(
                    keyringd_fido::webauthn::AuthenticatorSelectionCriteria::default()
                        .with_resident_key(ResidentKeyRequirement::Required)
                        .with_require_resident_key(true),
                );
                let expected_options = options.clone();
                let parameters = std::sync::Arc::new(|_: &[u8]| {
                    Ok(ClientData::from_bytes(vec![3]))
                });
                let operation_client = Arc::clone(&client);
                let mut operation =
                    tokio::spawn(
                        async move { operation_client.make_credential(options, keyringd_fido_client::MakeCredentialContext::new((parameters).as_ref())).await },
                    );
        }
        when "Call WAIT_FOR_ACTIVE_BACKEND" {
            let operation_id = tokio::select! {
                result = &mut operation => panic!("operation ended before platform USB started: {result:?}"),
                operation_id = wait_for_active_backend(&backend) => operation_id,
            };
        }
        then "Verify that the results match FidoClientError::Cancelled" {
            assert!(client.cancel(operation_id));
            assert_fido_error!(operation.await.unwrap(), FidoClientError::Cancelled);
            assert_eq!(*backend.cancelled.lock().unwrap(), [operation_id]);
            assert_eq!(
                *backend.make_credential_client_data.lock().unwrap(),
                Some(vec![3])
            );
            assert_eq!(
                *backend.make_credential_algorithms.lock().unwrap(),
                Some(vec![-7])
            );
            let received_options = backend
                .make_credential_options
                .lock()
                .unwrap()
                .clone()
                .expect("platform backend received creation options");
            assert!(matches!(received_options.timeout(), Some(9_000..=10_000)));
            assert_eq!(received_options.rp(), expected_options.rp());
            assert_eq!(received_options.user(), expected_options.user());
            assert_eq!(received_options.challenge(), expected_options.challenge());
            assert_eq!(
                received_options.public_key_credential_parameters(),
                expected_options.public_key_credential_parameters()
            );
            assert_eq!(
                received_options.exclude_credentials(),
                expected_options.exclude_credentials()
            );
            assert_eq!(
                received_options.authenticator_selection(),
                expected_options.authenticator_selection()
            );
            assert_eq!(received_options.hints(), expected_options.hints());
            assert_eq!(received_options.attestation(), expected_options.attestation());
            assert_eq!(
                received_options.attestation_formats(),
                expected_options.attestation_formats()
            );
            assert_eq!(received_options.extensions(), expected_options.extensions());
            assert_eq!(
                *handler.events.lock().unwrap(),
                [
                    ("started", operation_id),
                    ("backend_finished", operation_id),
                ]
            );
        }

        }
        case "Returns Busy from makeCredential while getAssertion is running" {
            spec "https://www.w3.org/TR/webauthn-3/#sctn-op-make-cred"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare the ongoing Get Assertion and subsequent Make Credentials" {
                let handler = Arc::new(LifecycleCalls::default());
                let backend = Arc::new(OsBackendCalls::default());
                let backend_contract = pending_os_backend(backend.clone());
                let client = Arc::new(os_test_client(lifecycle_handler(handler), backend_contract));
                let assertion_options = PublicKeyCredentialRequestOptions::new(vec![1])
                    .with_relying_party_id("example.com")
                    .with_timeout(10_000);
                let assertion_parameters = std::sync::Arc::new(
                    |_: &[u8]| Ok(ClientData::from_bytes(vec![2])),
                );
                let assertion_client = Arc::clone(&client);
                let assertion = tokio::spawn(async move {
                    assertion_client
                        .get_assertions(assertion_options, keyringd_fido_client::GetAssertionsContext::new((assertion_parameters).as_ref()))
                        .await
                });
                let assertion_operation_id = wait_for_active_backend(&backend).await;

                let creation_options = PublicKeyCredentialCreationOptions::new(
                    PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                    PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                    vec![3],
                    vec![PublicKeyCredentialParameters::es256()],
                )
                .with_timeout(10_000)
                .with_authenticator_selection(
                    keyringd_fido::webauthn::AuthenticatorSelectionCriteria::default()
                        .with_resident_key(ResidentKeyRequirement::Required),
                );
                let creation_parameters = std::sync::Arc::new(
                    |_: &[u8]| Ok(ClientData::from_bytes(vec![4])),
                );
                let creation_client = Arc::clone(&client);
            }
            when "Request additional operations" {
                let result = creation_client
                    .make_credential(creation_options, keyringd_fido_client::MakeCredentialContext::new(creation_parameters.as_ref()))
                    .await;
            }
            then "Returns BUSY and continues running operations" {
                assert_fido_error!(result, FidoClientError::Busy);
                assert!(!assertion.is_finished());
                assert!(backend.cancelled.lock().unwrap().is_empty());
                assert_eq!(*backend.active.lock().unwrap(), Some(assertion_operation_id));
                assert!(client.cancel(assertion_operation_id));
                assert_fido_error!(assertion.await.unwrap(), FidoClientError::Cancelled);
            }
        }
        case "Returns Busy from getAssertion while makeCredential is running" {
            spec "https://www.w3.org/TR/webauthn-3/#sctn-op-get-assertion"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare the ongoing Make Credential and subsequent Get Assertions" {
                let handler = Arc::new(LifecycleCalls::default());
                let backend = Arc::new(OsBackendCalls::default());
                let backend_contract = pending_os_backend(backend.clone());
                let client = Arc::new(os_test_client(lifecycle_handler(handler), backend_contract));
                let creation_options = PublicKeyCredentialCreationOptions::new(
                    PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                    PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                    vec![2],
                    vec![PublicKeyCredentialParameters::es256()],
                )
                .with_timeout(10_000)
                .with_authenticator_selection(
                    keyringd_fido::webauthn::AuthenticatorSelectionCriteria::default()
                        .with_resident_key(ResidentKeyRequirement::Required),
                );
                let creation_parameters = std::sync::Arc::new(
                    |_: &[u8]| Ok(ClientData::from_bytes(vec![3])),
                );
                let creation_client = Arc::clone(&client);
                let creation = tokio::spawn(async move {
                    creation_client
                        .make_credential(creation_options, keyringd_fido_client::MakeCredentialContext::new((creation_parameters).as_ref()))
                        .await
                });
                let creation_operation_id = wait_for_active_backend(&backend).await;

                let assertion_options = PublicKeyCredentialRequestOptions::new(vec![4])
                    .with_relying_party_id("example.com")
                    .with_timeout(10_000);
                let assertion_parameters = std::sync::Arc::new(
                    |_: &[u8]| Ok(ClientData::from_bytes(vec![5])),
                );
                let assertion_client = Arc::clone(&client);
            }
            when "Request additional operations" {
                let result = assertion_client
                    .get_assertions(assertion_options, keyringd_fido_client::GetAssertionsContext::new(assertion_parameters.as_ref()))
                    .await;
            }
            then "Returns BUSY and continues running operations" {
                assert_fido_error!(result, FidoClientError::Busy);
                assert!(!creation.is_finished());
                assert!(backend.cancelled.lock().unwrap().is_empty());
                assert_eq!(*backend.active.lock().unwrap(), Some(creation_operation_id));
                assert!(client.cancel(creation_operation_id));
                assert_fido_error!(creation.await.unwrap(), FidoClientError::Cancelled);
            }
        }
        case "Rejects WebAuthn creation options that omit the required user names" {
            spec "https://www.w3.org/TR/webauthn-3/#dictdef-publickeycredentialuserentity"
            async: tokio;

            given "creation options containing a CTAP-style ID-only user entity" {
                let client = FidoClient::new(usb_only_handler()).with_usb_backend(crate::backend::UsbBackend::Hid(std::sync::Arc::new(empty_usb_backend()))).with_hybrid_connector(crate::backend::SystemHybridChannelConnector::default());
                let options = PublicKeyCredentialCreationOptions::new(
                    PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                    PublicKeyCredentialUserEntity::from_id(vec![1]),
                    vec![2],
                    vec![PublicKeyCredentialParameters::es256()],
                )
                .with_authenticator_selection(
                    keyringd_fido::webauthn::AuthenticatorSelectionCriteria::default()
                        .with_resident_key(ResidentKeyRequirement::Required),
                );
                let parameters = std::sync::Arc::new(|_: &[u8]| {
                    Ok(ClientData::from_bytes(vec![3]))
                });
            }
            when "the high-level client starts credential creation" {
                let result = client.make_credential(options, keyringd_fido_client::MakeCredentialContext::new((parameters).as_ref())).await;
            }
            then "the missing WebAuthn user name is reported as a protocol error instead of panicking" {
                let error = result.expect_err("ID-only users are not valid WebAuthn creation options");
                assert!(matches!(&error, FidoClientError::Protocol(_)));
                assert!(matches!(std::error::Error::source(&error)
                        .and_then(|source| source.downcast_ref::<keyringd_fido::ProtocolError>()), Some(&keyringd_fido::ProtocolError::MissingField("user.name"))));
            }
        }
}

keyringd_testkit::scenario! {
    case "Rejects user handles outside the one-to-64-byte range before starting an authenticator operation" {
        spec "https://www.w3.org/TR/webauthn-3/#sctn-createCredential"
        async: tokio;
        timeout: std::time::Duration::from_secs(5);

        given "credential creation options with an empty user handle and with a 65-byte user handle" {
            let invalid_user_ids = [Vec::new(), vec![0; 65]];
            let client = FidoClient::new(usb_only_handler()).with_usb_backend(crate::backend::UsbBackend::Hid(std::sync::Arc::new(empty_usb_backend()))).with_hybrid_connector(crate::backend::SystemHybridChannelConnector::default());
        }
        when "each request is submitted to the high-level client" {
            let mut results = Vec::new();
            for user_id in invalid_user_ids {
                let options = PublicKeyCredentialCreationOptions::new(
                    PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                    PublicKeyCredentialUserEntity::new("user", user_id, "User"),
                    vec![1],
                    vec![PublicKeyCredentialParameters::es256()],
                )
                .with_authenticator_selection(
                    keyringd_fido::webauthn::AuthenticatorSelectionCriteria::default()
                        .with_resident_key(ResidentKeyRequirement::Required),
                );
                let parameters = std::sync::Arc::new(|_: &[u8]| {
                    Ok(ClientData::from_bytes(vec![2]))
                });
                results.push(client.make_credential(options, keyringd_fido_client::MakeCredentialContext::new((parameters).as_ref())).await);
            }
        }
        then "both requests fail as invalid WebAuthn input before transport selection" {
            for (actual, result) in [0, 65].into_iter().zip(results) {
                let error = result.expect_err("an invalid user handle must be rejected");
                assert!(matches!(&error, FidoClientError::Protocol(_)));
                assert!(matches!(
                    std::error::Error::source(&error)
                        .and_then(|source| source.downcast_ref::<keyringd_fido::ProtocolError>()),
                    Some(keyringd_fido::ProtocolError::EmptyValue { field: "user.id" }) if actual == 0
                ) || matches!(
                    std::error::Error::source(&error)
                        .and_then(|source| source.downcast_ref::<keyringd_fido::ProtocolError>()),
                    Some(keyringd_fido::ProtocolError::InvalidValue {
                        field: "user.id", expected: "no more than 64 bytes",
                    }) if actual != 0
                ));
            }

            for valid_length in [1, 64] {
                let options = PublicKeyCredentialCreationOptions::new(
                    PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                    PublicKeyCredentialUserEntity::new("user", vec![0; valid_length], "User"),
                    vec![1],
                    vec![PublicKeyCredentialParameters::es256()],
                );
                let options = options.with_authenticator_selection(
                    keyringd_fido::webauthn::AuthenticatorSelectionCriteria::default()
                        .with_resident_key(ResidentKeyRequirement::Required),
                );
                let context = PreparedMakeCredential::new(options, (std::sync::Arc::new(|_: &[u8]| Ok(ClientData::from_bytes(vec![2])))).as_ref());
                assert!(context.is_ok());
            }
        }
    }

    case "Uses default credential parameters when the relying party supplies an empty list" {
        spec "https://www.w3.org/TR/webauthn-3/#sctn-createCredential"

        given "otherwise valid discoverable-credential options with no public-key credential parameters" {
            let options = PublicKeyCredentialCreationOptions::new(
                PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                vec![2],
                Vec::new(),
            )
            .with_authenticator_selection(
                keyringd_fido::webauthn::AuthenticatorSelectionCriteria::default()
                    .with_resident_key(ResidentKeyRequirement::Required),
            );
        }
        when "the client resolves the effective credential parameters" {
            let parameters = PreparedMakeCredential::resolve_credential_parameters(&options).unwrap();
        }
        then "the supported ES256 default is selected" {
            assert_eq!(parameters, [PublicKeyCredentialParameters::es256()]);
        }
    }

    case "Ignores unknown credential types while retaining a supported ES256 parameter" {
        spec "https://www.w3.org/TR/webauthn-3/#sctn-createCredential"

        given "credential parameters containing an unknown type followed by public-key ES256, and another list containing only the unknown type" {
            let unknown = PublicKeyCredentialParameters::new(
                PublicKeyCredentialType::Unknown("future-key".to_owned()),
                keyringd_fido::webauthn::CoseAlgorithmIdentifier::ES256,
            );
            let mixed_options = PublicKeyCredentialCreationOptions::new(
                PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                vec![2],
                vec![
                    unknown.clone(),
                    PublicKeyCredentialParameters::es256(),
                ],
            )
            .with_authenticator_selection(
                keyringd_fido::webauthn::AuthenticatorSelectionCriteria::default()
                    .with_resident_key(ResidentKeyRequirement::Required),
            );
            let unsupported_options = PublicKeyCredentialCreationOptions::new(
                PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                vec![2],
                vec![unknown],
            );
            let parameters = std::sync::Arc::new(|_: &[u8]| {
                Ok(ClientData::from_bytes(vec![3]))
            });
        }
        when "credential creation is started and the unsupported-only list is resolved" {
            let result = PreparedMakeCredential::new(mixed_options, (parameters).as_ref());
            let unsupported = PreparedMakeCredential::resolve_credential_parameters(&unsupported_options);
        }
        then "the unknown parameter is ignored when ES256 remains and an empty effective list is rejected" {
            assert!(result.is_ok());
            assert!(matches!(&unsupported.expect_err("no supported credential parameters remain"), FidoClientError::UnsupportedRequirement(_)));
        }
    }

    case "Ignores an unknown credential protection policy client extension input" {
        spec "https://www.w3.org/TR/webauthn-3/#sctn-extension-request-parameters"

        given "typed creation options containing an unknown credential protection policy with enforcement requested" {
            let options = PublicKeyCredentialCreationOptions::new(
                PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                vec![2],
                vec![PublicKeyCredentialParameters::es256()],
            )
            .with_extensions(
                keyringd_fido::webauthn::AuthenticationExtensionsClientInputs::default()
                    .with_credential_protection_policy(
                        keyringd_fido::webauthn::CredentialProtectionPolicy::Unknown(
                            "futurePolicy".to_owned(),
                        ),
                    )
                    .with_enforce_credential_protection_policy(true),
            );
        }
        when "the client resolves registration extensions" {
            let resolved = PreparedMakeCredential::new(options.with_authenticator_selection(
                    keyringd_fido::webauthn::AuthenticatorSelectionCriteria::default()
                        .with_resident_key(ResidentKeyRequirement::Required),
                ), (std::sync::Arc::new(|_: &[u8]| Ok(ClientData::from_bytes(vec![3])))).as_ref()).unwrap();
        }
        then "the invalid extension and its enforcement flag are both ignored" {
            assert!(resolved.ctap_request().extensions().is_none());
            assert!(resolved.authenticator_requirements().required_extensions.is_empty());
        }
    }

    case "Applies the legacy resident-key flag when the resident-key value is unknown" {
        spec "https://www.w3.org/TR/webauthn-3/#dom-authenticatorselectioncriteria-residentkey"

        given "otherwise valid options whose unknown residentKey is accompanied by requireResidentKey true" {
            let options = PublicKeyCredentialCreationOptions::new(
                PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                vec![2],
                vec![PublicKeyCredentialParameters::es256()],
            )
            .with_authenticator_selection(
                keyringd_fido::webauthn::AuthenticatorSelectionCriteria::default()
                    .with_resident_key(ResidentKeyRequirement::new("future-value"))
                    .with_require_resident_key(true),
            );
            let parameters = std::sync::Arc::new(|_: &[u8]| {
                Ok(ClientData::from_bytes(vec![3]))
            });
        }
        when "credential creation is started" {
            let result = PreparedMakeCredential::new(options, (parameters).as_ref());
        }
        then "the unknown value is ignored and the legacy true flag makes discoverability required" {
            assert!(result.is_ok());
        }
    }

    case "Treats an unknown user-verification value as preferred" {
        spec "https://www.w3.org/TR/webauthn-3/#dom-authenticatorselectioncriteria-userverification"

        verify "the effective value is the WebAuthn default" {
            assert_eq!(
                PreparedAssertion::new(
                    PublicKeyCredentialRequestOptions::new(vec![1])
                        .with_relying_party_id("example.com")
                        .with_user_verification(UserVerificationRequirement::new("future-value")), (Arc::new(|_: &[u8]| Ok(ClientData::from_bytes(vec![2])))).as_ref(),
                ).unwrap().user_verification(),
                &UserVerificationRequirement::Preferred
            );
        }
    }

    case "Ignores unknown descriptor types unless every allowed credential is ignored" {
        spec "https://www.w3.org/TR/webauthn-3/#dom-publickeycredentialdescriptor-type"

        given "one allow list mixing an unknown type with public-key and another containing only the unknown type" {
            let unknown = PublicKeyCredentialDescriptor::new(
                PublicKeyCredentialType::Unknown("future-key".to_owned()),
                vec![1],
            );
            let supported = PublicKeyCredentialDescriptor::new(
                PublicKeyCredentialType::PublicKey,
                vec![2],
            );
        }
        when "both authentication requests are started" {
            let mixed = PreparedAssertion::new(PublicKeyCredentialRequestOptions::new(vec![3])
                    .with_relying_party_id("example.com")
                    .with_allow_credentials(vec![unknown.clone(), supported]), (std::sync::Arc::new(|_: &[u8]| Ok(ClientData::from_bytes(vec![4])))).as_ref());
            let all_unknown = PreparedAssertion::new(PublicKeyCredentialRequestOptions::new(vec![3])
                    .with_relying_party_id("example.com")
                    .with_allow_credentials(vec![unknown]), (std::sync::Arc::new(|_: &[u8]| Ok(ClientData::from_bytes(vec![4])))).as_ref());
        }
        then "the mixed list proceeds with public-key while the all-ignored list is rejected" {
            assert!(mixed.is_ok());
            assert!(matches!(&all_unknown.err().expect("the effective allow list is empty"), FidoClientError::UnsupportedRequirement(_)));
        }
    }

    case "Ignores unknown descriptor types in excludeCredentials" {
        spec "https://www.w3.org/TR/webauthn-3/#dom-publickeycredentialdescriptor-type"

        given "an exclude list containing an unknown type and a public-key descriptor" {
            let options = PublicKeyCredentialCreationOptions::new(
                PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                vec![2],
                vec![PublicKeyCredentialParameters::es256()],
            )
            .with_exclude_credentials(vec![
                PublicKeyCredentialDescriptor::new(
                    PublicKeyCredentialType::Unknown("future-key".to_owned()),
                    vec![3],
                ),
                PublicKeyCredentialDescriptor::new(
                    PublicKeyCredentialType::PublicKey,
                    vec![4],
                ),
            ]);
        }
        when "the client resolves the exclude list" {
            let resolved = PreparedMakeCredential::resolve_exclude_credentials(&options);
        }
        then "only the public-key descriptor remains" {
            assert_eq!(resolved.len(), 1);
            assert_eq!(resolved[0].credential_type(), &PublicKeyCredentialType::PublicKey);
            assert_eq!(resolved[0].id(), &[4]);
        }
    }

    case "Does not reject an unknown transport hint on a public-key descriptor" {
        spec "https://www.w3.org/TR/webauthn-3/#dom-publickeycredentialdescriptor-transports"

        given "an allowed public-key credential with only a future transport hint" {
            let descriptor = PublicKeyCredentialDescriptor::new(
                PublicKeyCredentialType::PublicKey,
                vec![1],
            )
            .with_transports(vec![AuthenticatorTransport::new("future-transport")]);
            let options = PublicKeyCredentialRequestOptions::new(vec![2])
                .with_relying_party_id("example.com")
                .with_allow_credentials(vec![descriptor]);
            let parameters = std::sync::Arc::new(|_: &[u8]| {
                Ok(ClientData::from_bytes(vec![3]))
            });
        }
        when "authentication is started" {
            let result = PreparedAssertion::new(options, (parameters).as_ref());
        }
        then "the unknown hint is ignored rather than rejected as an unsupported requirement" {
            assert!(result.is_ok());
        }
    }

    case "Passes exact getAssertion inputs to the OS USB backend" {
        spec "https://www.w3.org/TR/webauthn-3/#sctn-discover-from-external-source"
        async: tokio;
        timeout: std::time::Duration::from_secs(5);

        given "Prepare the OS USB Backend and Discoverable Credential Authentication Option to record the input" {
            let handler = Arc::new(LifecycleCalls::default());
            let backend = Arc::new(OsBackendCalls::default());
            let client = Arc::new(os_test_client(
                lifecycle_handler(handler),
                pending_os_backend(backend.clone()),
            ));
            let signed_client_data = br#"{"type":"webauthn.get", "challenge":"AQ"}"#.to_vec();
            let options = PublicKeyCredentialRequestOptions::new(vec![1])
                .with_timeout(10_000)
                .with_relying_party_id("example.com")
                .with_user_verification(UserVerificationRequirement::Required);
            let expected_options = options.clone();
            let expected_client_data = signed_client_data.clone();
            let parameters = std::sync::Arc::new(move |_: &[u8]| {
                Ok(ClientData::from_bytes(expected_client_data.clone()))
            });
            let operation_client = Arc::clone(&client);
            let mut operation = tokio::spawn(async move {
                operation_client.get_assertions(options, keyringd_fido_client::GetAssertionsContext::new((parameters).as_ref())).await
            });
        }
        when "Wait until Platform Backend is called" {
            let operation_id = tokio::select! {
                result = &mut operation => panic!("operation ended before platform USB started: {result:?}"),
                operation_id = wait_for_active_backend(&backend) => operation_id,
            };
        }
        then "RP ID, UV, Extension, ClientData passed to the Authenticator boundary" {
            assert!(client.cancel(operation_id));
            assert_fido_error!(operation.await.unwrap(), FidoClientError::Cancelled);
            let received = backend
                .get_assertions_options
                .lock()
                .unwrap()
                .clone()
                .expect("platform backend received assertion options");
            assert!(matches!(received.timeout(), Some(9_000..=10_000)));
            assert_eq!(received.challenge(), expected_options.challenge());
            assert_eq!(received.relying_party_id(), expected_options.relying_party_id());
            assert_eq!(received.allow_credentials(), expected_options.allow_credentials());
            assert_eq!(received.user_verification(), expected_options.user_verification());
            assert_eq!(received.hints(), expected_options.hints());
            assert_eq!(received.extensions(), expected_options.extensions());
            assert_eq!(
                *backend.get_assertions_client_data.lock().unwrap(),
                Some(signed_client_data)
            );
        }
    }

    case "Treat unknown attestation values as unspecified" {
        spec "https://www.w3.org/TR/webauthn-3/#dom-publickeycredentialcreationoptions-attestation"
        async: tokio;
        timeout: std::time::Duration::from_secs(5);

        given "Prepare a Credential Create Option with unknown attestation values" {
            let handler = Arc::new(LifecycleCalls::default());
            let backend = Arc::new(OsBackendCalls::default());
            let client = Arc::new(os_test_client(
                lifecycle_handler(handler),
                pending_os_backend(backend.clone()),
            ));
            let options = PublicKeyCredentialCreationOptions::new(
                PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                vec![2],
                vec![PublicKeyCredentialParameters::es256()],
            )
            .with_timeout(10_000)
            .with_authenticator_selection(
                keyringd_fido::webauthn::AuthenticatorSelectionCriteria::default()
                    .with_resident_key(ResidentKeyRequirement::Required),
            )
            .with_attestation(AttestationConveyancePreference::new("future-attestation"));
            let parameters = std::sync::Arc::new(|_: &[u8]| {
                Ok(ClientData::from_bytes(vec![3]))
            });
            let operation_client = Arc::clone(&client);
            let mut operation = tokio::spawn(async move {
                operation_client.make_credential(options, keyringd_fido_client::MakeCredentialContext::new((parameters).as_ref())).await
            });
        }
        when "Wait until Platform Backend is called" {
            let operation_id = tokio::select! {
                result = &mut operation => panic!("operation ended before platform USB started: {result:?}"),
                operation_id = wait_for_active_backend(&backend) => operation_id,
            };
        }
        then "The unknown value is passed to the Authenticator boundary as the default value of WebAuthn None" {
            assert!(client.cancel(operation_id));
            assert_fido_error!(operation.await.unwrap(), FidoClientError::Cancelled);
            let received = backend
                .make_credential_options
                .lock()
                .unwrap()
                .clone()
                .expect("platform backend received creation options");
            assert_eq!(received.attestation(), &AttestationConveyancePreference::None);
        }
    }

    case "Treat the duplicate Credential Generation Parameters as a single case" {
        spec "https://www.w3.org/TR/webauthn-3/#dom-publickeycredentialcreationoptions-pubkeycredparams"

        given "Prepare an Option with the same Public-Key ES256 Parameter" {
            let options = PublicKeyCredentialCreationOptions::new(
                PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                vec![2],
                vec![
                    PublicKeyCredentialParameters::es256(),
                    PublicKeyCredentialParameters::es256(),
                ],
            );
        }
        when "Resolve a valid Credential generation parameter" {
            let resolved = PreparedMakeCredential::resolve_credential_parameters(&options).unwrap();
        }
        then "Duplicates are ignored, only one ES256 remains" {
            assert_eq!(resolved, [PublicKeyCredentialParameters::es256()]);
        }
    }

    case "Preserve Priority Order of Allow Credentials" {
        spec "https://www.w3.org/TR/webauthn-3/#dom-publickeycredentialrequestoptions-allowcredentials"

        given "Prepare two public-key descriptors in order of priority" {
            let preferred = PublicKeyCredentialDescriptor::new(
                PublicKeyCredentialType::PublicKey,
                vec![1],
            );
            let fallback = PublicKeyCredentialDescriptor::new(
                PublicKeyCredentialType::PublicKey,
                vec![2],
            );
            let options = PublicKeyCredentialRequestOptions::new(vec![3])
                .with_allow_credentials(vec![preferred.clone(), fallback.clone()]);
        }
        when "Resolve valid Allow Credentials" {
            let resolved = crate::client::PreparedAssertion::resolve_allow_credentials(&options).unwrap().unwrap();
        }
        then "The order of input with the top priority is preserved" {
            assert_eq!(resolved, [preferred, fallback]);
        }
    }
}

mod operation;
