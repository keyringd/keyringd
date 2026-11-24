use crate::ClientData;
use crate::FidoClientError;
use crate::client::PreparedAssertion;
use crate::client::PreparedMakeCredential;
use crate::client::operation::OperationId;
use crate::client::operation::OperationManager;
use crate::client::operation::ctap::CtapOperationExecutor;
use crate::client::operation::ctap::CtapUserVerification;
use crate::client::operation::ctap::ResponseValidationError;
use crate::client::operation::ctap::validate_discoverable_user;
use crate::client::operation::test_support::BrokerTestClient;
use crate::client::operation::test_support::ModelHybridConnector;
use crate::client::operation::test_support::RecordingHandler;
use crate::client::operation::test_support::built_in_uv_transport;
use crate::client::operation::test_support::make_options;
use crate::client::operation::test_support::model_capabilities;
use crate::client::operation::test_support::puat_required_once_transport;
use crate::client::operation::test_support::test_context;
use crate::ctap::AuthenticatorCapabilities;
use crate::ctap::AuthenticatorRequest;
use crate::ctap::AuthenticatorRequirements;
use crate::ctap::pin_uv::PinUvAuthProtocol;
use crate::test_support::MockCtapTransport;
use crate::test_support::assert_fido_error;
use crate::test_support::model_transport;
use crate::tests::support::client_operation_authenticator_broker::assert_protocol_error;
use crate::tests::support::client_operation_authenticator_broker::fixed_get_info_transport;
use crate::tests::support::client_operation_authenticator_broker::run_interrupted_hybrid_ctap_operation;
use futures_util::future::BoxFuture;
use futures_util::stream::FuturesUnordered;
use keyringd_fido::AuthenticatorTransport;
use keyringd_fido::UserVerificationRequirement;
use keyringd_fido::ctap::AuthenticatorGetAssertionRequest;
use keyringd_fido::ctap::AuthenticatorGetInfoResponse;
use keyringd_fido::ctap::AuthenticatorMakeCredentialRequest;
use keyringd_fido::ctap::ClientDataHash;
use keyringd_fido::ctap::PinUvAuthPermissions;
use keyringd_fido::webauthn::AuthenticationExtensionsClientInputs;
use keyringd_fido::webauthn::CredentialProtectionPolicy;
use keyringd_fido::webauthn::PublicKeyCredentialDescriptor;
use keyringd_fido::webauthn::PublicKeyCredentialParameters;
use keyringd_fido::webauthn::PublicKeyCredentialRequestOptions;
use keyringd_fido::webauthn::PublicKeyCredentialRpEntity;
use keyringd_fido::webauthn::PublicKeyCredentialType;
use keyringd_fido::webauthn::PublicKeyCredentialUserEntity;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

keyringd_testkit::scenario! {
        case "Client Cancel targets only registered Operations" {
            spec "https://www.w3.org/TR/webauthn-3/#sctn-abortoperation"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare a running operation" {
                let service = Arc::new(OperationManager::new(RecordingHandler::default()));
                let active_service = Arc::clone(&service);
                let (started_sender, started) = tokio::sync::oneshot::channel();
                let active = tokio::spawn(async move {
                    active_service.run_operation(Instant::now() + Duration::from_secs(1), |context| async move {
                        started_sender.send(context.id).unwrap();
                        context.cancellation.cancelled().await;
                        Err::<(), _>(FidoClientError::Cancelled)
                    }).await
                });
                let operation_id = started.await.unwrap();
            }
            when "Cancel the target operation" {
                let unknown_cancelled = service.cancel(OperationId::new(43));
                let active_cancelled = service.cancel(operation_id);
                let result = active.await.unwrap();
            }
            then "A non-existent operation cannot cancel and only the running operation cancel" {
                assert!(!unknown_cancelled);
                assert!(active_cancelled);
            }
            then "Only the registered operations are canceled and the registration is canceled after the end." {
                assert_fido_error!(result, FidoClientError::Cancelled);
                assert!(!service.cancel(operation_id));
                assert!(!service.cancel(OperationId::new(43)));
            }
        }
        case "Request User Entity from Discoverable Assertion" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"

            given "Omit User Entity for both Discoverable and Allow List requests" {
                let discoverable_allow_list = None;
                let allow_list = Some(vec![PublicKeyCredentialDescriptor::new(
                    PublicKeyCredentialType::PublicKey,
                    vec![1],
                )]);
        }
        when "Validate User Entity Requirements" {
            let discoverable_result = validate_discoverable_user(discoverable_allow_list, None);
            let allow_list_result = validate_discoverable_user(allow_list.as_deref(), None);
        }
        then "Only Discoverable requests are rejected as missing User Entity" {
            let error = discoverable_result.unwrap_err();
            assert_protocol_error::<ResponseValidationError>(&error, |error| {
                matches!(
                    error,
                    ResponseValidationError::MissingDiscoverableUserEntity
                )
            });
            assert!(allow_list_result.is_ok());
        }

        }
        case "Enforces credential ID length and list size limits" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

            given "Prepare PublicKeyCredentialDescriptor, PublicKeyCredentialType, AuthenticatorMakeCredentialRequest" {
                let descriptor =
                    |id: Vec<u8>| PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, id);
                let mut capabilities = model_capabilities();
                capabilities = AuthenticatorCapabilities::from_get_info(
                    capabilities.info().clone().with_max_credential_id_length(2),
                );
                capabilities = AuthenticatorCapabilities::from_get_info(
                    capabilities
                        .info()
                        .clone()
                        .with_max_credential_count_in_list(1),
                );
            }
        when "Call PublicKeyCredentialParameters::es256" {
            let make_request = AuthenticatorMakeCredentialRequest::new(
                ClientDataHash::new([0; 32]),
                PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                vec![PublicKeyCredentialParameters::es256()],
            )
            .with_exclude_list(vec![descriptor(vec![1]), descriptor(vec![2, 3, 4])]);
            let constrained = capabilities.constrain_make_credential_request(make_request).map_err(FidoClientError::from).unwrap();
        }
        then "Verify that the results match FidoClientError::UnsupportedRequirement" {
            assert_eq!(constrained.exclude_list().unwrap()[0].id(), [1]);
            let get_request =
                AuthenticatorGetAssertionRequest::new("example.com", ClientDataHash::new([0; 32]))
                    .with_allow_list(vec![descriptor(vec![1]), descriptor(vec![2])]);
            assert_fido_error!(capabilities.constrain_get_assertion_request(get_request).map_err(FidoClientError::from), FidoClientError::UnsupportedRequirement(_));
        }

        }
        case "Returns No Credentials if all Allow IDs are excluded" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"

            given "Prepare AuthenticatorGetAssertionRequest, ClientDataHash, PublicKeyCreddentialDescriptor" {
                let mut capabilities = model_capabilities();
                capabilities = AuthenticatorCapabilities::from_get_info(capabilities.info().clone().with_max_credential_id_length(2));
        }
        when "Call Constrain_Get_Assertion_Request" {
            let request =
                AuthenticatorGetAssertionRequest::new("example.com", ClientDataHash::new([0; 32]))
                    .with_allow_list(vec![PublicKeyCredentialDescriptor::new(
                        PublicKeyCredentialType::PublicKey,
                        vec![1, 2, 3],
                    )]);
            let error = capabilities.constrain_get_assertion_request(request).map_err(FidoClientError::from).unwrap_err();
        }
        then "Verify that the results match CtapStatusCode::NoCredentials" {
            assert!(matches!(&error, FidoClientError::Ctap(keyringd_fido::CtapStatusCode::NoCredentials)));
        }

        }
        case "Compare the final encoded request with the Maximum Message Size" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#message-encoding"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

            given "Prepare FidoClientError, Error, RecordingHandler" {
                let mut capabilities = model_capabilities();
        }
        when "Set 3 to Capabilities.Maximum_Message_Size" {
            capabilities = AuthenticatorCapabilities::from_get_info(capabilities.info().clone().with_max_message_size(3));
        }
        then "Verify that the results match FidoClientError::UnsupportedRequirement" {
            assert_fido_error!(capabilities.ensure_message_size(4).map_err(FidoClientError::from), FidoClientError::UnsupportedRequirement(_));
            assert!(capabilities.ensure_message_size(3).map_err(FidoClientError::from).is_ok());
        }

        }
        case "Excessive operations return busy and continue existing operations" {
            spec "https://www.w3.org/TR/webauthn-3/#sctn-abortoperation"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Start the operation and hold the post-cleaning after cancellation" {
                let handler = Arc::new(RecordingHandler::default());
                let service = Arc::new(OperationManager::new(Arc::clone(&handler)));
                let (started_sender, started) = tokio::sync::oneshot::channel();
                let (cleanup_sender, cleanup) = tokio::sync::oneshot::channel();
                let active_service = Arc::clone(&service);
                let active = tokio::spawn(async move {
                    active_service.run_operation(Instant::now() + Duration::from_secs(2), |context| async move {
                        started_sender.send((context.id, context.cancellation.clone())).unwrap();
                        context.cancellation.cancelled().await;
                        cleanup.await.unwrap();
                        Err::<(), _>(FidoClientError::Cancelled)
                    }).await
                });
                let (operation_id, cancellation) = started.await.unwrap();
            }
            when "Request additional operations" {
                let result = service.run_operation::<(), _, _>(Instant::now() + Duration::from_secs(1), |_| {
                    panic!("In the case of busy, no processing is performed");
                    #[allow(unreachable_code)]
                    std::future::ready(Ok(()))
                }).await;
            }
            then "BUSY refuses additional operations until the end of the cleanup is completed without canceling the existing operation." {
                assert_fido_error!(result, FidoClientError::Busy);
                assert!(!active.is_finished());
                assert!(!cancellation.is_cancelled());
                assert_eq!(*handler.events.lock().unwrap(), ["started"]);
                assert!(service.cancel(operation_id));
                assert_fido_error!(service.run_operation(Instant::now() + Duration::from_secs(1), |_| async { Ok(()) }).await, FidoClientError::Busy);
                cleanup_sender.send(()).unwrap();
                assert_fido_error!(active.await.unwrap(), FidoClientError::Cancelled);
                assert!(!service.cancel(operation_id));
                assert!(service.run_operation(Instant::now() + Duration::from_secs(1), |_| async { Ok(()) }).await.is_ok());
                assert_eq!(*handler.events.lock().unwrap(), ["started", "backend_finished", "started", "backend_finished"]);
            }
        }
        case "AuthenticatorBroker completes Noise and CTAP with Model Authenticator" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare ModelCtapAuthenticator/ModelHybridConnector/RecordingHandler" {
                let authenticator = Arc::new(keyringd_test_authenticator::ModelCtapAuthenticator::new());
                let closed_channels = Arc::new(AtomicUsize::new(0));
                let shutdown_frames = Arc::new(AtomicUsize::new(0));
                let connector = Arc::new(ModelHybridConnector {
                    authenticator,
                    closed_channels: Arc::clone(&closed_channels),
                    shutdown_frames: Arc::clone(&shutdown_frames),
                    ctap_entered: None,
                    resume_ctap: None,
                });
                let handler = Arc::new(RecordingHandler::default());
                let service = BrokerTestClient::with_hybrid_connector(handler, connector);
                let options = make_options(UserVerificationRequirement::Discouraged, 1);
        }
        when "Call make_credential" {
            let outcome = service
                .make_credential(options)
                .await
                .unwrap();
            let credential = outcome;
        }
        then "AuthenticatorBroker completes Noise and CTAP with Model Authenticator" {
            assert!(!credential.credential().raw_id().is_empty());
            assert_eq!(closed_channels.load(Ordering::SeqCst), 1);
            assert_eq!(shutdown_frames.load(Ordering::SeqCst), 1);
        }

        }
        case "Send shutdown in Cancel during Hybrid CTAP to close Tunnel" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-hybrid"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            verify "Send shutdown in Cancel during Hybrid CTAP to close Tunnel" {
                run_interrupted_hybrid_ctap_operation(true).await;
                }

        }
        case "Send Shutdown on Timeout during Hybrid CTAP and close Tunnel" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-hybrid"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            verify "Send Shutdown on Timeout during Hybrid CTAP and close Tunnel" {
                run_interrupted_hybrid_ctap_operation(false).await;
                }

        }
        case "Omits the uv option when the authenticator has no built-in user verification" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorMakeCredential"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "MakeCredential and GetAssertion requests plus capabilities without a selected verification method" {
                let authenticator = std::sync::Arc::new(
                    keyringd_test_authenticator::ModelCtapAuthenticator::new(),
                );
                let mut verification_client = crate::CtapClient::new(Box::new(model_transport(authenticator)));
let verification_transport = AuthenticatorTransport::usb();
let verification_capabilities = model_capabilities();
let mut authenticator = CtapUserVerification::new(&mut verification_client, &verification_capabilities, &verification_transport, &UserVerificationRequirement::Preferred).unwrap();
                let handler = RecordingHandler::default();
                let operation_id = OperationId::new(27);
                let make_options = make_options(UserVerificationRequirement::Preferred, 1);
                let mut make_request = AuthenticatorMakeCredentialRequest::new(
                    ClientDataHash::hash(make_options.client_data.as_bytes()),
                    make_options.ctap_request().relying_party().clone(),
                    make_options.ctap_request().user().clone(),
                    make_options.ctap_request().public_key_credential_parameters().to_vec(),
                )
                .with_option("rk", true);
                let mut get_request = AuthenticatorGetAssertionRequest::new(
                    "ssh:keyringd",
                    ClientDataHash::new([0x42; 32]),
                )
                .with_option("up", true);
            }
            when "the client applies the selected verification method to both requests" {
                authenticator.apply_make_user_verification(&test_context(&handler, operation_id), &mut make_request)
                .await
                .unwrap();
                authenticator.apply_get_user_verification(&test_context(&handler, operation_id), &mut get_request, false)
                .await
                .unwrap();
            }
            then "neither request contains the uv option" {
                assert_eq!(
                    make_request.options().and_then(|options| options.get("uv")),
                    None
                );
                assert_eq!(
                    get_request.options().and_then(|options| options.get("uv")),
                    None
                );
            }
        }
        case "Retries GetAssertion with a pinUvAuthParam after CTAP2_ERR_PUAT_REQUIRED" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorClientPIN"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "an authenticator that requires a PIN/UV auth token after an initially unverified request" {
                let authenticator = std::sync::Arc::new(
                    keyringd_test_authenticator::ModelCtapAuthenticator::with_pin_uv_auth_token(
                        vec![0x5a; 32],
                    ),
                );
                let handler = RecordingHandler::default();
                let operation_id = OperationId::new(19);
                let make_options = make_options(UserVerificationRequirement::Discouraged, 1);
                let credential = CtapOperationExecutor::new(
                    model_transport(Arc::clone(&authenticator)),
                    AuthenticatorTransport::usb(),
                    model_capabilities(),
                )
                .make_credential(&crate::MakeCredentialOperationContext::new(&make_options, test_context(&handler, operation_id), None, None))
                .await
                .unwrap();
                let allow_list = vec![PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, credential.credential().raw_id().to_vec())];
                let options = PreparedAssertion::new(PublicKeyCredentialRequestOptions::new(vec![2])
            .with_relying_party_id("ssh:keyringd").with_allow_credentials(allow_list.clone())
            .with_user_verification(UserVerificationRequirement::Discouraged), (std::sync::Arc::new(|_: &[u8]| Ok(ClientData::from_bytes(vec![2])))).as_ref()).unwrap();
                let options = options.with_user_presence(true);
                let attempts = Arc::new(AtomicUsize::new(0));
        }
        when "the client receives CTAP2_ERR_PUAT_REQUIRED and obtains a permission-scoped token using built-in UV" {
            let capabilities = AuthenticatorCapabilities::from_get_info(model_capabilities().info().clone().with_options(BTreeMap::from([("rk".to_owned(), true),("uv".to_owned(), true),("pinUvAuthToken".to_owned(), true)])).with_pin_uv_auth_protocols(vec![2]));
            let assertions = CtapOperationExecutor::new(
                puat_required_once_transport(authenticator, Arc::clone(&attempts), crate::ctap::test_support::TestPinUvKeyAgreement::new(
                        PinUvAuthProtocol::Two,
                    ), true, PinUvAuthPermissions::GET_ASSERTION),
                AuthenticatorTransport::usb(),
                capabilities,
            )
            .get_assertions(&crate::GetAssertionOperationContext::new(&options, test_context(&handler, operation_id), None, None))
            .await
            .unwrap();
        }
        then "GetAssertion is retried once with pinUvAuthParam and succeeds" {
            assert_eq!(assertions.responses().len(), 1);
            assert_eq!(attempts.load(Ordering::SeqCst), 2);
        }

        }
        case "Uses getPinUvAuthTokenUsingUvWithPermissions before the first GetAssertion attempt" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#gettingPinUvAuthToken"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getassert-uv"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "a discoverable credential and an authenticator selected for permission-scoped built-in UV" {
                let authenticator = std::sync::Arc::new(
                    keyringd_test_authenticator::ModelCtapAuthenticator::with_pin_uv_auth_token(
                        vec![0x5a; 32],
                    ),
                );
                let handler = RecordingHandler::default();
                let operation_id = OperationId::new(22);
                let make_options = make_options(UserVerificationRequirement::Discouraged, 1);
                let credential = CtapOperationExecutor::new(
                    model_transport(Arc::clone(&authenticator)),
                    AuthenticatorTransport::usb(),
                    model_capabilities(),
                )
                .make_credential(&crate::MakeCredentialOperationContext::new(&make_options, test_context(&handler, operation_id), None, None))
                .await
                .unwrap();
                let allow_list = vec![PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, credential.credential().raw_id().to_vec())];
                let options = PreparedAssertion::new(PublicKeyCredentialRequestOptions::new(vec![2])
            .with_relying_party_id("ssh:keyringd").with_allow_credentials(allow_list.clone())
            .with_user_verification(UserVerificationRequirement::Required), (std::sync::Arc::new(|_: &[u8]| Ok(ClientData::from_bytes(vec![2])))).as_ref()).unwrap();
                let options = options.with_user_presence(true);
                let attempts = Arc::new(AtomicUsize::new(0));
                let capabilities = AuthenticatorCapabilities::from_get_info(model_capabilities().info().clone().with_options(BTreeMap::from([("rk".to_owned(), true),("uv".to_owned(), true),("pinUvAuthToken".to_owned(), true)])).with_pin_uv_auth_protocols(vec![2]));
            }
            when "the client executes GetAssertion on the selected authenticator" {
                let assertions = CtapOperationExecutor::new(
                    puat_required_once_transport(authenticator, Arc::clone(&attempts), crate::ctap::test_support::TestPinUvKeyAgreement::new(
                            PinUvAuthProtocol::Two,
                        ), false, PinUvAuthPermissions::GET_ASSERTION),
                    AuthenticatorTransport::usb(),
                    capabilities,
                )
                .get_assertions(&crate::GetAssertionOperationContext::new(&options, test_context(&handler, operation_id), None, None))
                .await
                .unwrap();
            }
            then "the first and only GetAssertion request contains pinUvAuthParam instead of the uv option" {
                assert_eq!(assertions.responses().len(), 1);
                assert_eq!(attempts.load(Ordering::SeqCst), 1);
            }
        }
        case "Uses getPinUvAuthTokenUsingUvWithPermissions before MakeCredential" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#gettingPinUvAuthToken"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#makecred-uv"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "a MakeCredential operation and an authenticator selected for permission-scoped built-in UV" {
                let authenticator = std::sync::Arc::new(
                    keyringd_test_authenticator::ModelCtapAuthenticator::with_pin_uv_auth_token(
                        vec![0x5a; 32],
                    ),
                );
                let handler = RecordingHandler::default();
                let operation_id = OperationId::new(23);
                let options = make_options(UserVerificationRequirement::Required, 1);
                let capabilities = AuthenticatorCapabilities::from_get_info(model_capabilities().info().clone().with_options(BTreeMap::from([("rk".to_owned(), true),("uv".to_owned(), true),("pinUvAuthToken".to_owned(), true)])).with_pin_uv_auth_protocols(vec![2]));
            }
            when "the client executes MakeCredential on the selected authenticator" {
                let credential = CtapOperationExecutor::new(
                    puat_required_once_transport(authenticator, Arc::new(AtomicUsize::new(0)), crate::ctap::test_support::TestPinUvKeyAgreement::new(
                            PinUvAuthProtocol::Two,
                        ), false, PinUvAuthPermissions::MAKE_CREDENTIAL),
                    AuthenticatorTransport::usb(),
                    capabilities,
                )
                .make_credential(&crate::MakeCredentialOperationContext::new(&options, test_context(&handler, operation_id), None, None))
                .await
                .unwrap();
            }
            then "the credential is created with pinUvAuthParam instead of the uv option" {
                assert!(!credential.credential().raw_id().is_empty());
            }
        }
        case "High-level Native Execution Creates and Gets All Model Credentials" {
            spec "https://www.w3.org/TR/webauthn-3/#sctn-createCredential"
            spec "https://www.w3.org/TR/webauthn-3/#sctn-discover-from-external-source"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare ModelCtapAuthenticator/RecordingHandler/OperationId" {
                let authenticator =
                    std::sync::Arc::new(keyringd_test_authenticator::ModelCtapAuthenticator::new());
                let handler = RecordingHandler::default();
                let operation_id = OperationId::new(20);
                let mut created = Vec::new();
                for user_id in [1, 2] {
                    let options = make_options(UserVerificationRequirement::Discouraged, user_id);
                    created.push(
                        CtapOperationExecutor::new(
                            model_transport(std::sync::Arc::clone(&authenticator)),
                            AuthenticatorTransport::usb(),
                            model_capabilities(),
                        )
                        .make_credential(&crate::MakeCredentialOperationContext::new(&options, test_context(&handler, operation_id), None, None))
                        .await
                        .unwrap(),
                    );
                }
                let options = PreparedAssertion::new(PublicKeyCredentialRequestOptions::new(vec![9])
            .with_relying_party_id("ssh:keyringd")
            .with_user_verification(UserVerificationRequirement::Discouraged), (std::sync::Arc::new(|_: &[u8]| Ok(ClientData::from_bytes(vec![9])))).as_ref()).unwrap();
        }
        when "Call ClientDataHash::hash" {
            let options = options.with_user_presence(true);
            let assertions = CtapOperationExecutor::new(
                model_transport(authenticator),
                AuthenticatorTransport::usb(),
                model_capabilities(),
            )
            .get_assertions(&crate::GetAssertionOperationContext::new(&options, test_context(&handler, OperationId::new(21)), None, None))
            .await
            .unwrap();
        }
        then "High-level Native Execution Creates and Gets All Model Credentials" {
            assert_eq!(created.len(), 2);
            assert_eq!(assertions.responses().len(), 2);
            assert!(assertions.responses().iter().all(|value| {
                created
                    .iter()
                    .any(|credential| credential.credential().raw_id() == value.credential().id())
            }));
        }

        }
        case "Cancel Selected Channels in Cancel during CTAP Operation" {
            spec "https://www.w3.org/TR/webauthn-3/#sctn-abortoperation"
            spec "https://www.w3.org/TR/webauthn-3/#sctn-op-cancel"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare ResidentKeyRequirement, UserVerificationRequirement, and AuthenticatorMakeCredentialRequest" {
                let entered = Arc::new(AtomicBool::new(false));
                let options = make_options(UserVerificationRequirement::Discouraged, 1);
                let cancellation = tokio_util::sync::CancellationToken::new();
                let operation_cancellation = cancellation.clone();
                let operation_entered = Arc::clone(&entered);

                let operation = tokio::spawn(async move {
                    CtapOperationExecutor::new(
                        {
                            let mut transport = MockCtapTransport::new();
                            transport.expect_transmit().times(1).return_once(move |_| {
                                Box::pin(async move {
                                    operation_entered.store(true, Ordering::SeqCst);
                                    std::future::pending().await
                                })
                            });
                            transport.expect_cancel().times(1).returning(|| Box::pin(async { Ok(()) }));
                            transport.expect_close().never();
                            transport
                        },
                        AuthenticatorTransport::usb(),
                        model_capabilities(),
                    )
                    .make_credential(&crate::MakeCredentialOperationContext::new(&options, test_context(&RecordingHandler::default(), OperationId::new(30))
                            .with_cancellation(operation_cancellation), None, None)).await
                });
                while !entered.load(Ordering::SeqCst) {
                    tokio::task::yield_now().await;
                }
        }
        when "Call Cancellation.Cancel" {
            cancellation.cancel();
        }
        then "Verify that the results match FidoClientError::Cancelled" {
            assert_fido_error!(operation.await.unwrap(), FidoClientError::Cancelled);
        }

        }
    group "Built-in user verification retry and fallback" {
            case "Checks UV retries and retries built-in UV within the authenticator's preferred attempt budget" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-preferredplatformuvattempts"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getPinUvAuthTokenUsingUvWithPermissions"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "an authenticator that reports one failed built-in UV attempt, remaining retries, and a two-attempt preference" {
                    let authenticator = Arc::new(
                        keyringd_test_authenticator::ModelCtapAuthenticator::with_pin_uv_auth_token(
                            vec![0x5a; 32],
                        ),
                    );
                    let subcommands = Arc::new(Mutex::new(Vec::new()));
                    let handler = RecordingHandler::default();
                    let operation_id = OperationId::new(24);
                    let options = make_options(UserVerificationRequirement::Required, 1);
                    let capabilities = AuthenticatorCapabilities::from_get_info(model_capabilities().info().clone().with_options(BTreeMap::from([("rk".to_owned(), true),("uv".to_owned(), true),("pinUvAuthToken".to_owned(), true)])).with_pin_uv_auth_protocols(vec![2]).with_preferred_platform_uv_attempts(2));
                }
                when "MakeCredential obtains its token using built-in UV" {
                    let credential = CtapOperationExecutor::new(
                        built_in_uv_transport(authenticator, crate::ctap::test_support::TestPinUvKeyAgreement::new(
                                PinUvAuthProtocol::Two,
                            ), Arc::clone(&subcommands), 1, keyringd_fido::CtapStatusCode::UserVerificationInvalid, 3),
                        AuthenticatorTransport::usb(),
                        capabilities,
                    )
                    .make_credential(&crate::MakeCredentialOperationContext::new(&options, test_context(&handler, operation_id), None, None))
                    .await
                    .unwrap();
                }
                then "the client queries getUVRetries and succeeds on the second built-in UV attempt without requesting a PIN" {
                    assert!(!credential.credential().raw_id().is_empty());
                    assert_eq!(
                        *subcommands.lock().unwrap(),
                        [
                            keyringd_fido::ctap::ClientPinSubcommand::GetKeyAgreement,
                            keyringd_fido::ctap::ClientPinSubcommand::GetPinUvAuthTokenUsingUvWithPermissions,
                            keyringd_fido::ctap::ClientPinSubcommand::GetUvRetries,
                            keyringd_fido::ctap::ClientPinSubcommand::GetKeyAgreement,
                            keyringd_fido::ctap::ClientPinSubcommand::GetPinUvAuthTokenUsingUvWithPermissions,
                        ]
                    );
                }
            }
            case "Falls back to ClientPIN when built-in UV has no retries remaining" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#gettingPinUvAuthToken"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getPinUvAuthTokenUsingUvWithPermissions"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "an authenticator with both built-in UV and ClientPIN whose failed UV attempt leaves zero retries" {
                    let authenticator = Arc::new(
                        keyringd_test_authenticator::ModelCtapAuthenticator::with_pin_uv_auth_token(
                            vec![0x5a; 32],
                        ),
                    );
                    let subcommands = Arc::new(Mutex::new(Vec::new()));
                    let handler = RecordingHandler {
                        pin: Some("1234"),
                        ..RecordingHandler::default()
                    };
                    let operation_id = OperationId::new(25);
                    let options = make_options(UserVerificationRequirement::Required, 1);
                    let capabilities = AuthenticatorCapabilities::from_get_info(model_capabilities().info().clone().with_options(BTreeMap::from([("rk".to_owned(), true),("uv".to_owned(), true),("pinUvAuthToken".to_owned(), true),("clientPin".to_owned(), true)])).with_pin_uv_auth_protocols(vec![2]).with_preferred_platform_uv_attempts(1));
                }
                when "MakeCredential obtains its user-verification token" {
                    let credential = CtapOperationExecutor::new(
                        built_in_uv_transport(authenticator, crate::ctap::test_support::TestPinUvKeyAgreement::new(
                                PinUvAuthProtocol::Two,
                            ), Arc::clone(&subcommands), 1, keyringd_fido::CtapStatusCode::UserVerificationInvalid, 0),
                        AuthenticatorTransport::usb(),
                        capabilities,
                    )
                    .make_credential(&crate::MakeCredentialOperationContext::new(&options, test_context(&handler, operation_id), None, None))
                    .await
                    .unwrap();
                }
                then "the client checks UV retries and obtains a permission-scoped token with the entered PIN" {
                    assert!(!credential.credential().raw_id().is_empty());
                    assert_eq!(
                        *subcommands.lock().unwrap(),
                        [
                            keyringd_fido::ctap::ClientPinSubcommand::GetKeyAgreement,
                            keyringd_fido::ctap::ClientPinSubcommand::GetPinUvAuthTokenUsingUvWithPermissions,
                            keyringd_fido::ctap::ClientPinSubcommand::GetUvRetries,
                            keyringd_fido::ctap::ClientPinSubcommand::GetPinRetries,
                            keyringd_fido::ctap::ClientPinSubcommand::GetKeyAgreement,
                            keyringd_fido::ctap::ClientPinSubcommand::GetPinUvAuthTokenUsingPinWithPermissions,
                        ]
                    );
                }
            }
            case "Falls back to ClientPIN immediately when built-in UV is blocked" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getPinUvAuthTokenUsingUvWithPermissions"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "an authenticator with blocked built-in UV and an available ClientPIN method" {
                    let authenticator = Arc::new(
                        keyringd_test_authenticator::ModelCtapAuthenticator::with_pin_uv_auth_token(
                            vec![0x5a; 32],
                        ),
                    );
                    let subcommands = Arc::new(Mutex::new(Vec::new()));
                    let handler = RecordingHandler {
                        pin: Some("1234"),
                        ..RecordingHandler::default()
                    };
                    let mut verification_client = crate::CtapClient::new(Box::new(built_in_uv_transport(authenticator, crate::ctap::test_support::TestPinUvKeyAgreement::new(
                            PinUvAuthProtocol::Two,
                        ), Arc::clone(&subcommands), 1, keyringd_fido::CtapStatusCode::UserVerificationBlocked, 0)));
let verification_transport = AuthenticatorTransport::usb();
let verification_capabilities = AuthenticatorCapabilities::from_get_info(model_capabilities().info().clone().with_options(BTreeMap::from([("rk".to_owned(), true),("uv".to_owned(), true),("pinUvAuthToken".to_owned(), true),("clientPin".to_owned(), true)])).with_pin_uv_auth_protocols(vec![2]).with_preferred_platform_uv_attempts(2));
let mut authenticator = CtapUserVerification::new(&mut verification_client, &verification_capabilities, &verification_transport, &UserVerificationRequirement::Required).unwrap();
                }
                when "the client requests a permission-scoped user-verification token" {
                    let mut request = make_options(UserVerificationRequirement::Required, 1).ctap_request().clone();
                    let result = authenticator.apply_make_user_verification(&test_context(&handler, OperationId::new(26)), &mut request)
                    .await;
                }
                then "the client skips getUVRetries and obtains the token through ClientPIN" {
                    assert!(result.is_ok());
                    assert_eq!(
                        *subcommands.lock().unwrap(),
                        [
                            keyringd_fido::ctap::ClientPinSubcommand::GetKeyAgreement,
                            keyringd_fido::ctap::ClientPinSubcommand::GetPinUvAuthTokenUsingUvWithPermissions,
                            keyringd_fido::ctap::ClientPinSubcommand::GetPinRetries,
                            keyringd_fido::ctap::ClientPinSubcommand::GetKeyAgreement,
                            keyringd_fido::ctap::ClientPinSubcommand::GetPinUvAuthTokenUsingPinWithPermissions,
                        ]
                    );
                }
            }
    }
    group "MakeCredential credential-protection requirements" {
        case "Preserves an enforced relying-party credential-protection policy" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#userverificationoptional"

            given "level-two and level-three relying-party policies with enforcement enabled" {
                let policies = [
                    CredentialProtectionPolicy::UserVerificationOptionalWithCredentialIdList,
                    CredentialProtectionPolicy::UserVerificationRequired,
                ];
            }
            when "each policy is converted into a CTAP request and authenticator requirements" {
                let requests = policies.map(|policy| {
                    let options = make_options(UserVerificationRequirement::Preferred, 1);
                    PreparedMakeCredential::new(options.options.with_extensions(AuthenticationExtensionsClientInputs::default()
                            .with_credential_protection_policy(policy)
                            .with_enforce_credential_protection_policy(true)), (std::sync::Arc::new(|_: &[u8]| Ok(ClientData::from_bytes(vec![1])))).as_ref()).unwrap()
                });
            }
            then "the exact policy value is retained and authenticators without credProtect are excluded" {
                for (options, expected_value) in requests.into_iter().zip([2, 3]) {
                    assert_eq!(options.authenticator_requirements().required_extensions, ["credProtect"]);
                    let request = options.ctap_request();
                    assert_eq!(
                        request.extensions().unwrap().values()["credProtect"].as_bytes(),
                        [expected_value]
                    );
                }
            }
        }
    }
    group "USB authenticator candidate selection" {
            case "Reports that the authenticator PIN must be changed when no suitable candidate remains" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-forcepinchange"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "an authenticator that requires client-PIN verification and advertises forcePINChange" {
                    let info = AuthenticatorGetInfoResponse::new(
                        vec!["FIDO_2_1".to_owned()],
                        [0; 16],
                    )
                    .with_options(BTreeMap::from([
                        ("clientPin".to_owned(), true),
                        ("rk".to_owned(), true),
                    ]))
                    .with_pin_uv_auth_protocols(vec![2])
                    .with_force_pin_change(true)
                    .with_min_pin_length(4);
                    let requirements = AuthenticatorRequirements {
                        discoverable_credential: true,
                        user_verification: UserVerificationRequirement::Required,
                        algorithms: vec![
                            keyringd_fido::webauthn::CoseAlgorithmIdentifier::ES256,
                        ],
                        required_extensions: Vec::new(),
                    };
                    let candidates: FuturesUnordered<
                        BoxFuture<'static, Result<CtapOperationExecutor, FidoClientError>>,
                    > = FuturesUnordered::new();
                    candidates.push(Box::pin(async move {
                        CtapOperationExecutor::connect(Box::new(fixed_get_info_transport(info)),
AuthenticatorTransport::usb(),
&requirements,
AuthenticatorRequest::GetAssertion(&AuthenticatorGetAssertionRequest::new("example.com", ClientDataHash::new([0; 32]))),
tokio_util::sync::CancellationToken::new(), None)
                        .await
                    }));
                }
                when "all connected USB candidates have been evaluated" {
                    let result = futures_util::StreamExt::next(&mut { candidates }).await.unwrap();
                }
                then "the caller receives the actionable PIN-change error instead of a generic unsupported or missing-authenticator result" {
                    let Err(error) = result else {
                        panic!("a forcePINChange authenticator must not be selected");
                    };
                    assert!(matches!(&error, FidoClientError::PinChangeRequired(_)));
                }
            }
    }
}

keyringd_testkit::scenario! {
    case "Omits the up option when sending a request to a CTAP 2.0 authenticator" {
        spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#makecred-uv"
        ignored: "deferred to prioritize completing the CTAP 2.2 client";

        given "a MakeCredential request with the platform's user-presence option and capabilities discovered from a CTAP 2.0 authenticator" {
            let request = AuthenticatorMakeCredentialRequest::new(
                ClientDataHash::new([0; 32]),
                PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                vec![PublicKeyCredentialParameters::es256()],
            )
            .with_option("up", true);
            let requirements = AuthenticatorRequirements {
                discoverable_credential: true,
                user_verification: UserVerificationRequirement::Discouraged,
                algorithms: vec![
                    keyringd_fido::webauthn::CoseAlgorithmIdentifier::ES256,
                ],
                required_extensions: Vec::new(),
            };
            let info = AuthenticatorGetInfoResponse::new(
                vec!["FIDO_2_0".to_owned()],
                [0; 16],
            );
            let capabilities = AuthenticatorCapabilities::from_get_info(info);
            capabilities.check_requirements(&requirements, AuthenticatorRequest::MakeCredential(&request)).unwrap();
        }
        when "the request is constrained for the selected authenticator" {
            let constrained = capabilities.constrain_make_credential_request(request).map_err(FidoClientError::from).unwrap();
        }
        then "the request sent to the CTAP 2.0 authenticator does not contain the up option" {
            assert!(
                constrained
                    .options()
                    .is_none_or(|options| !options.contains_key("up"))
            );
        }
    }
}
