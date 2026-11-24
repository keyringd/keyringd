use crate::ctap::CHANNEL_BUSY_RETRY_DELAY;
use crate::ctap::CtapClient;
use crate::ctap::CtapClientError;
use crate::ctap::MAX_CHANNEL_BUSY_RETRIES;
use crate::ctap::PinUvAuthProtocol;
use crate::ctap::test_support::TestPinUvKeyAgreement;
use crate::ctap::test_support::pin_transport;
use crate::ctap::test_support::recording_transport;
use crate::ctap::test_support::set_pin_transport;
use crate::test_support::model_transport;
use crate::tests::support::ctap_ctap_client::assertion;
use crate::tests::support::ctap_ctap_client::credential_material;
use crate::tests::support::ctap_ctap_client::make_credential_request;
use crate::tests::support::ctap_ctap_client::request;
use crate::tests::support::ctap_ctap_client::success;
use crate::tests::support::ctap_ctap_client::verify_assertion_signature;
use keyringd_fido::CtapCommand;
use keyringd_fido::CtapRequest;
use keyringd_fido::CtapStatusCode;
use keyringd_fido::ctap::AuthenticatorClientPinRequest;
use keyringd_fido::ctap::AuthenticatorClientPinResponse;
use keyringd_fido::ctap::AuthenticatorGetAssertionRequest;
use keyringd_fido::ctap::ClientDataHash;
use keyringd_fido::ctap::ClientPinSubcommand;
use keyringd_fido::ctap::CtapVersion;
use keyringd_fido::ctap::Pin;
use keyringd_fido::ctap::PinUvAuthPermissions;
use keyringd_fido::webauthn::PublicKeyCredentialDescriptor;
use sha2::Digest;
use sha2::Sha256;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

keyringd_testkit::scenario! {
        case "Maintain the Authenticator Status" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#error-responses"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare MockCtapTransport, vecdeque, and ctapstatuscode" {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let channel = recording_transport(VecDeque::from([vec![CtapStatusCode::PinInvalid.as_byte()]]), commands.clone(), Default::default());
        }
        when "Generate a ctappclient from a channel that returns a PIN_INVALID response" {
            let mut client = CtapClient::new(Box::new(channel));
        }
        then "Verify that the results match CtapClientError::Status and CtapStatusCode::PinInvalid" {
            assert!(matches!(
                client.get_assertion(&request(), CtapVersion::Fido21).await,
                Err(CtapClientError::Status(CtapStatusCode::PinInvalid))
            ));
        }
        }
        case "Get PIN UV Auth Token associated with Permission" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorClientPIN"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare Session with Permission for Protocol 1 and 2 with PIN" {
                let protocols = [PinUvAuthProtocol::One, PinUvAuthProtocol::Two];
        }
        when "Get PIN UV Auth Token for Get Assertion on each Protocol" {
            let mut results = Vec::new();
            for protocol in protocols {

                let mut client = CtapClient::new(Box::new(pin_transport(TestPinUvKeyAgreement::new(protocol), true, false)));
                let token = client
                    .pin_uv_auth()
                    .get_pin_uv_auth_token_using_pin(
                        protocol,
                        &Pin::from_text("1234").unwrap(),
                        PinUvAuthPermissions::GET_ASSERTION,
                        Some("example.com".to_owned()),
                    )
                    .await
                    .unwrap();
                results.push((
                    protocol,
                    token.protocol(),
                    token.authenticate(&[7; 32]).unwrap().len(),
                ));
            }
        }
        then "Returns Token with Authentication Value Length per Protocol Complete 2 Client PIN Exchanges" {
            assert_eq!(
                results,
                [
                    (PinUvAuthProtocol::One, PinUvAuthProtocol::One, 16),
                    (PinUvAuthProtocol::Two, PinUvAuthProtocol::Two, 32),
                ]
            );
        }

        }
        case "Get CTAP 2_0 PIN Token without Permission" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorClientPIN"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare PIN Session for Protocol 1 and 2 without Permission" {
                let protocols = [PinUvAuthProtocol::One, PinUvAuthProtocol::Two];
        }
        when "Get CTAP 2_0 Compatible PIN TOKEN on each Protocol" {
            let mut clients = Vec::new();
            for protocol in protocols {

                let mut client = CtapClient::new(Box::new(pin_transport(TestPinUvKeyAgreement::new(protocol), false, false)));
                client
                    .pin_uv_auth()
                    .get_pin_token_using_pin(protocol, &Pin::from_text("1234").unwrap())
                    .await
                    .unwrap();
                clients.push(client);
            }
        }
        then "Complete 2 Client PIN exchanges on both protocols" {
            drop(clients); // Mockall verifies the number and order of key exchange/token acquisition.
        }

        }
        case "Get PIN UV Auth Token tied to Permission using UV" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorClientPIN"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare a UV Session with Permission for Protocol 1 and 2 using UV" {
                let protocols = [PinUvAuthProtocol::One, PinUvAuthProtocol::Two];
        }
        when "Get tokens for Get Assertions by UV on each protocol" {
            let mut clients = Vec::new();
            for protocol in protocols {

                let mut client = CtapClient::new(Box::new(pin_transport(TestPinUvKeyAgreement::new(protocol), true, true)));
                client
                    .pin_uv_auth()
                    .get_pin_uv_auth_token_using_uv(
                        protocol,
                        PinUvAuthPermissions::GET_ASSERTION,
                        Some("example.com".to_owned()),
                    )
                    .await
                    .unwrap();
                clients.push(client);
            }
        }
        then "Complete 2 Client PIN exchanges on both protocols" {
            drop(clients); // Mockall verifies the number and order of key exchange/token acquisition.
        }

        }
    group "PinUvAuth::get_pin_retries" {
            case "Sends getPINRetries with the negotiated PIN UV protocol" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#gettingPINRetries"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a CTAP channel that records the ClientPIN request and reports eight remaining attempts" {
                    let mut transport = crate::test_support::MockCtapTransport::new();
                    transport.expect_transmit().times(1).withf(|request| {
                        request.command() == CtapCommand::ClientPin &&
                            AuthenticatorClientPinRequest::from_cbor_bytes(request.payload()).unwrap() ==
                                AuthenticatorClientPinRequest::new(ClientPinSubcommand::GetPinRetries)
                                    .with_pin_uv_auth_protocol(1)
                    }).returning(|_| Box::pin(async {
                        Ok(keyringd_fido::CtapResponse::new(CtapStatusCode::Ok,
                            AuthenticatorClientPinResponse::new().with_pin_retries(8).to_cbor_bytes().unwrap()))
                    }));
                    let mut client = CtapClient::new(Box::new(transport));
                }
                when "the platform queries the remaining PIN retries" {
                    let retries = client
                        .pin_uv_auth()
                        .get_pin_retries(PinUvAuthProtocol::One)
                        .await
                        .unwrap();
                }
                then "the request is accepted by CTAP 2.0 authenticators that require pinUvAuthProtocol" {
                    assert_eq!(retries, 8);
                    drop(client); // Mockall validates the request and one call.
                }
            }
    }
    group "PinUvAuth::set_pin" {
            case "Configures an unset Client PIN through the negotiated PIN UV protocol" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#settingNewPin"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a Client PIN channel implementing the negotiated key agreement" {

                    let mut client = CtapClient::new(Box::new(set_pin_transport(TestPinUvKeyAgreement::new(PinUvAuthProtocol::Two))));
                }
                when "the Client PIN protocol configures the first PIN" {
                    client
                        .pin_uv_auth()
                        .set_pin(PinUvAuthProtocol::Two, &Pin::from_text("1234").unwrap())
                        .await
                        .unwrap();
                }
                then "the authenticated 64-byte newPinEnc exchange completes" {
                    drop(client); // Mockall validates key exchange and PIN settings once in a sequence.
                }
            }

    }
    group "CtapClient with ModelCtapAuthenticator" {
            case "Creates discoverable ES256 credentials and verifies every returned assertion" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-discoverable"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetNextAssertion"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a model authenticator and two discoverable credentials for the same relying party" {
                    let authenticator = std::sync::Arc::new(
                        keyringd_test_authenticator::ModelCtapAuthenticator::new(),
                    );
                    let mut client = CtapClient::new(Box::new(model_transport(authenticator.clone())));
                    let mut credentials = std::collections::HashMap::new();
                    let mut creation_order = Vec::new();
                    for user in [1, 2] {
                        let response = client
                            .make_credential(&make_credential_request(
                                user,
                                "example.com",
                                true,
                                None,
                            ))
                            .await
                            .unwrap();
                        let (credential_id, verifying_key) = credential_material(&response);
                        creation_order.push(credential_id.clone());
                        credentials.insert(credential_id, (verifying_key, vec![user]));
                    }
                    let client_data_hash = ClientDataHash::new([7; 32]);
                }
                when "all discoverable assertions are requested without an allow list" {
                    let assertions = client
                        .get_assertions(&AuthenticatorGetAssertionRequest::new(
                            "example.com",
                            client_data_hash,
                        ), CtapVersion::Fido21)
                        .await
                        .unwrap();
                }
                then "the authenticator returns newest-first credentials with valid signatures and user handles" {
                    assert_eq!(assertions.len(), 2);
                    assert_eq!(
                        assertions
                            .iter()
                            .map(|assertion| assertion.credential().unwrap().id().to_vec())
                            .collect::<Vec<_>>(),
                        creation_order.into_iter().rev().collect::<Vec<_>>()
                    );
                    for assertion in &assertions {
                        assert_eq!(
                            assertion.authenticator_data().rp_id_hash(),
                            Sha256::digest(b"example.com").as_slice()
                        );
                        let (verifying_key, expected_user_handle) =
                            credentials.get(assertion.credential().unwrap().id()).unwrap();
                        assert_eq!(assertion.user().unwrap().id(), expected_user_handle);
                        verify_assertion_signature(
                            assertion,
                            &client_data_hash,
                            verifying_key,
                        );
                    }
                    assert_eq!(assertions[0].authenticator_data().sign_count(), 1);
                    assert_eq!(assertions[1].authenticator_data().sign_count(), 2);
                    assert_eq!(
                        &authenticator.commands()[2..],
                        &[CtapCommand::GetAssertion, CtapCommand::GetNextAssertion]
                    );
                }
            }
            case "Returns one assertion without GetNextAssertion when an allow list is present" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "two discoverable credentials and an allow list containing both credential IDs" {
                    let authenticator = std::sync::Arc::new(
                        keyringd_test_authenticator::ModelCtapAuthenticator::new(),
                    );
                    let mut client = CtapClient::new(Box::new(model_transport(authenticator.clone())));
                    let mut allowed = Vec::new();
                    for user in [1, 2] {
                        let response = client
                            .make_credential(&make_credential_request(
                                user,
                                "example.com",
                                true,
                                None,
                            ))
                            .await
                            .unwrap();
                        allowed.push(PublicKeyCredentialDescriptor::new(
                            keyringd_fido::webauthn::PublicKeyCredentialType::PublicKey,
                            credential_material(&response).0,
                        ));
                    }
                }
                when "assertions are requested using the allow list" {
                    let assertions = client
                        .get_assertions(
                            &AuthenticatorGetAssertionRequest::new(
                                "example.com",
                                ClientDataHash::new([7; 32]),
                            )
                            .with_allow_list(allowed), CtapVersion::Fido21)
                        .await
                        .unwrap();
                }
                then "the response contains one credential and no assertion-enumeration command" {
                    assert_eq!(assertions.len(), 1);
                    assert_eq!(assertions[0].number_of_credentials(), None);
                    assert_eq!(
                        authenticator.commands().last(),
                        Some(&CtapCommand::GetAssertion)
                    );
                    assert_eq!(
                        authenticator
                            .commands()
                            .iter()
                            .filter(|command| **command == CtapCommand::GetNextAssertion)
                            .count(),
                        0
                    );
                }
            }
            case "Requires an allow list to use a non-discoverable credential" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-discoverable"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a non-discoverable credential registered for the relying party" {
                    let authenticator = std::sync::Arc::new(
                        keyringd_test_authenticator::ModelCtapAuthenticator::new(),
                    );
                    let mut client = CtapClient::new(Box::new(model_transport(authenticator)));
                    let registration = client
                        .make_credential(&make_credential_request(
                            1,
                            "example.com",
                            false,
                            None,
                        ))
                        .await
                        .unwrap();
                    let credential_id = credential_material(&registration).0;
                }
                when "the credential is requested first by discovery and then by credential ID" {
                    let discovery = client.get_assertions(&request(), CtapVersion::Fido21).await;
                    let allowed = client
                        .get_assertions(
                            &request().with_allow_list(vec![PublicKeyCredentialDescriptor::new(
                                keyringd_fido::webauthn::PublicKeyCredentialType::PublicKey,
                                credential_id,
                            )]), CtapVersion::Fido21)
                        .await
                        .unwrap();
                }
                then "discovery reports no credential while the allow-listed request succeeds" {
                    assert!(matches!(
                        discovery,
                        Err(CtapClientError::Status(CtapStatusCode::NoCredentials))
                    ));
                    assert_eq!(allowed.len(), 1);
                    assert!(allowed[0].user().is_none());
                }
            }
            case "Replaces an existing discoverable credential for the same relying-party account" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-discoverable"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a model authenticator with a discoverable credential for one relying-party account" {
                    let authenticator = std::sync::Arc::new(
                        keyringd_test_authenticator::ModelCtapAuthenticator::new(),
                    );
                    let mut client = CtapClient::new(Box::new(model_transport(authenticator)));
                    let original = client
                        .make_credential(&make_credential_request(
                            1,
                            "example.com",
                            true,
                            None,
                        ))
                        .await
                        .unwrap();
                    let original_id = credential_material(&original).0;
                }
                when "another discoverable credential is created with the same RP ID and user handle" {
                    let replacement = client
                        .make_credential(&make_credential_request(
                            1,
                            "example.com",
                            true,
                            None,
                        ))
                        .await
                        .unwrap();
                    let replacement_id = credential_material(&replacement).0;
                    let assertions = client.get_assertions(&request(), CtapVersion::Fido21).await.unwrap();
                }
                then "discovery returns only the replacement credential" {
                    assert_ne!(replacement_id, original_id);
                    assert_eq!(assertions.len(), 1);
                    assert_eq!(assertions[0].credential().unwrap().id(), replacement_id);
                }
            }
            case "Enforces credential protection during discovery and allow-listed authentication" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-credProtect-extension"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "discoverable credentials protected at levels two and three" {
                    let authenticator = std::sync::Arc::new(
                        keyringd_test_authenticator::ModelCtapAuthenticator::new(),
                    );
                    let mut client = CtapClient::new(Box::new(model_transport(authenticator)));
                    let level_two_registration = client
                        .make_credential(&make_credential_request(
                            2,
                            "example.com",
                            true,
                            Some(2),
                        ))
                        .await
                        .unwrap();
                    let level_two_id = credential_material(&level_two_registration).0;
                    let level_three_registration = client
                        .make_credential(&make_credential_request(
                            3,
                            "example.com",
                            true,
                            Some(3),
                        ))
                        .await
                        .unwrap();
                    let level_three_id = credential_material(&level_three_registration).0;
                }
                when "the credentials are requested without verification, by allow list, and with verification" {
                    let discovery_without_uv = client.get_assertions(&request(), CtapVersion::Fido21).await;
                    let level_two_by_id = client
                        .get_assertions(
                            &request().with_allow_list(vec![PublicKeyCredentialDescriptor::new(
                                keyringd_fido::webauthn::PublicKeyCredentialType::PublicKey,
                                level_two_id,
                            )]), CtapVersion::Fido21)
                        .await
                        .unwrap();
                    let level_three_without_uv = client
                        .get_assertions(
                            &request().with_allow_list(vec![PublicKeyCredentialDescriptor::new(
                                keyringd_fido::webauthn::PublicKeyCredentialType::PublicKey,
                                level_three_id.clone(),
                            )]), CtapVersion::Fido21)
                        .await;
                    let level_three_with_uv = client
                        .get_assertions(
                            &request()
                                .with_allow_list(vec![PublicKeyCredentialDescriptor::new(
                                    keyringd_fido::webauthn::PublicKeyCredentialType::PublicKey,
                                    level_three_id,
                                )])
                                .with_option("uv", true), CtapVersion::Fido21)
                        .await
                        .unwrap();
                }
                then "level two requires an ID while level three additionally requires user verification" {
                    assert!(matches!(
                        discovery_without_uv,
                        Err(CtapClientError::Status(CtapStatusCode::NoCredentials))
                    ));
                    assert_eq!(level_two_by_id.len(), 1);
                    assert!(matches!(
                        level_three_without_uv,
                        Err(CtapClientError::Status(CtapStatusCode::NoCredentials))
                    ));
                    assert_eq!(level_three_with_uv.len(), 1);
                    assert!(level_three_with_uv[0].authenticator_data().user_verified());
                    assert_eq!(
                        level_two_registration
                            .authenticator_data()
                            .extensions()
                            .unwrap()
                            .values()["credProtect"]
                            .as_bytes(),
                        [2]
                    );
                    assert_eq!(
                        level_three_registration
                            .authenticator_data()
                            .extensions()
                            .unwrap()
                            .values()["credProtect"]
                            .as_bytes(),
                        [3]
                    );
                }
            }
            case "Protects level-three credentials from unauthenticated exclusion probing" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-credProtect-extension"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a level-three credential and MakeCredential requests that exclude its ID" {
                    let authenticator = std::sync::Arc::new(
                        keyringd_test_authenticator::ModelCtapAuthenticator::new(),
                    );
                    let mut client = CtapClient::new(Box::new(model_transport(authenticator)));
                    let protected = client
                        .make_credential(&make_credential_request(
                            1,
                            "example.com",
                            true,
                            Some(3),
                        ))
                        .await
                        .unwrap();
                    let protected_id = credential_material(&protected).0;
                    let excluded = PublicKeyCredentialDescriptor::new(
                        keyringd_fido::webauthn::PublicKeyCredentialType::PublicKey,
                        protected_id,
                    );
                    let unauthenticated_request = make_credential_request(
                        2,
                        "example.com",
                        true,
                        None,
                    )
                    .with_exclude_list(vec![excluded.clone()]);
                    let verified_request = make_credential_request(
                        3,
                        "example.com",
                        true,
                        None,
                    )
                    .with_exclude_list(vec![excluded])
                    .with_option("uv", true);
                }
                when "the exclusion check is performed without and then with user verification" {
                    let unauthenticated = client.make_credential(&unauthenticated_request).await;
                    let verified = client.make_credential(&verified_request).await;
                }
                then "the protected credential is hidden without UV and reported as excluded after UV" {
                    assert!(unauthenticated.is_ok());
                    assert!(matches!(
                        verified,
                        Err(CtapClientError::Status(CtapStatusCode::CredentialExcluded))
                    ));
                }
            }
            case "Rejects an invalid credential-protection extension value" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-credProtect-extension"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a MakeCredential request whose credProtect value is outside levels one through three" {
                    let authenticator = std::sync::Arc::new(
                        keyringd_test_authenticator::ModelCtapAuthenticator::new(),
                    );
                    let mut client = CtapClient::new(Box::new(model_transport(authenticator)));
                    let invalid_request = make_credential_request(
                        1,
                        "example.com",
                        true,
                        Some(4),
                    );
                }
                when "the credential is created" {
                    let result = client.make_credential(&invalid_request).await;
                }
                then "the authenticator returns InvalidOption and stores no credential" {
                    assert!(matches!(
                        result,
                        Err(CtapClientError::Status(CtapStatusCode::InvalidOption))
                    ));
                    assert!(matches!(
                        client.get_assertions(&request(), CtapVersion::Fido21).await,
                        Err(CtapClientError::Status(CtapStatusCode::NoCredentials))
                    ));
                }
            }
            case "Does not treat an unverified pinUvAuthParam as user verification" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto1"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a model authenticator without ClientPIN support and a request containing an arbitrary pinUvAuthParam" {
                    let authenticator = std::sync::Arc::new(
                        keyringd_test_authenticator::ModelCtapAuthenticator::new(),
                    );
                    let mut client = CtapClient::new(Box::new(model_transport(authenticator)));
                    let request = make_credential_request(
                        1,
                        "example.com",
                        true,
                        Some(3),
                    )
                    .with_pin_uv_auth(vec![0; 16], 2);
                }
                when "credential creation is attempted with the unauthenticated parameter" {
                    let result = client.make_credential(&request).await;
                }
                then "the model rejects the parameter instead of setting the UV flag" {
                    assert!(matches!(
                        result,
                        Err(CtapClientError::Status(CtapStatusCode::PinAuthInvalid))
                    ));
                }
            }
            case "Invalidates pending assertion enumeration when another command intervenes" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetNextAssertion"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "two discoverable credentials with one pending GetNextAssertion result" {
                    let authenticator = std::sync::Arc::new(
                        keyringd_test_authenticator::ModelCtapAuthenticator::new(),
                    );
                    let mut client = CtapClient::new(Box::new(model_transport(authenticator)));
                    let mut credential_ids = Vec::new();
                    for user in [1, 2] {
                        let registration = client
                            .make_credential(&make_credential_request(
                                user,
                                "example.com",
                                true,
                                None,
                            ))
                            .await
                            .unwrap();
                        credential_ids.push(credential_material(&registration).0);
                    }
                    let first = client.get_assertion(&request(), CtapVersion::Fido21).await.unwrap();
                    assert_eq!(first.number_of_credentials(), Some(2));
                }
                when "GetInfo is sent before GetNextAssertion" {
                    client.get_info().await.unwrap();
                    let next = client.get_next_assertion().await;
                    let later_assertion = client
                        .get_assertions(
                            &request().with_allow_list(vec![PublicKeyCredentialDescriptor::new(
                                keyringd_fido::webauthn::PublicKeyCredentialType::PublicKey,
                                credential_ids.remove(0),
                            )]), CtapVersion::Fido21)
                        .await
                        .unwrap();
                }
                then "the pending assertion is discarded without signing it" {
                    assert!(matches!(
                        next,
                        Err(CtapClientError::Status(CtapStatusCode::NotAllowed))
                    ));
                    assert_eq!(later_assertion[0].authenticator_data().sign_count(), 2);
                }
            }
            case "Rejects GetNextAssertion after the authenticator enumeration timeout" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetNextAssertion"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a model authenticator with an expired assertion timeout and two discoverable credentials" {
                    let authenticator = std::sync::Arc::new(
                        keyringd_test_authenticator::ModelCtapAuthenticator::with_assertion_timeout(
                            std::time::Duration::ZERO,
                        ),
                    );
                    let mut client = CtapClient::new(Box::new(model_transport(authenticator)));
                    for user in [1, 2] {
                        client
                            .make_credential(&make_credential_request(
                                user,
                                "example.com",
                                true,
                                None,
                            ))
                            .await
                            .unwrap();
                    }
                    let first = client.get_assertion(&request(), CtapVersion::Fido21).await.unwrap();
                    assert_eq!(first.number_of_credentials(), Some(2));
                }
                when "the client requests the next assertion after the deadline" {
                    let next = client.get_next_assertion().await;
                }
                then "the authenticator discards enumeration state and returns NotAllowed" {
                    assert!(matches!(
                        next,
                        Err(CtapClientError::Status(CtapStatusCode::NotAllowed))
                    ));
                }
            }
    }
    group "CtapClient::execute" {
            case "Retries a CTAP request after a channel-busy status" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#error-responses"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a channel that reports CTAP1_ERR_CHANNEL_BUSY once before returning a successful payload" {
                    let commands = Arc::new(Mutex::new(Vec::new()));
                    let channel = recording_transport(VecDeque::from([
                            vec![CtapStatusCode::ChannelBusy.as_byte()],
                            vec![CtapStatusCode::Ok.as_byte(), 0xa0],
                        ]), commands.clone(), Default::default());
                    let mut client = CtapClient::new(Box::new(channel));
                }
                when "the client executes a GetInfo request" {
                    let started = std::time::Instant::now();
                    let payload = client.execute(CtapRequest::new(CtapCommand::GetInfo, Vec::new())).await.unwrap();
                    let elapsed = started.elapsed();

                }
                then "the request is retried after a short delay and the successful payload is returned" {
                    assert_eq!(payload, vec![0xa0]);
                    assert!(elapsed >= CHANNEL_BUSY_RETRY_DELAY);
                    assert_eq!(
                        *commands.lock().unwrap(),
                        vec![CtapCommand::GetInfo, CtapCommand::GetInfo]
                    );
                }
            }
            case "Stops retrying CTAP channel-busy responses after a bounded retry budget" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#error-responses"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a channel that reports CTAP1_ERR_CHANNEL_BUSY for every request attempt" {
                    let commands = Arc::new(Mutex::new(Vec::new()));
                    let channel = recording_transport(VecDeque::from(
                            (0..=MAX_CHANNEL_BUSY_RETRIES)
                                .map(|_| vec![CtapStatusCode::ChannelBusy.as_byte()])
                                .collect::<Vec<_>>(),
                        ), commands.clone(), Default::default());
                    let mut client = CtapClient::new(Box::new(channel));
                }
                when "the client exhausts its retry budget" {
                    let result = client.execute(CtapRequest::new(CtapCommand::GetInfo, Vec::new())).await;

                }
                then "the final channel-busy status is returned instead of retrying indefinitely" {
                    assert!(matches!(
                        result,
                        Err(CtapClientError::Status(CtapStatusCode::ChannelBusy))
                    ));
                    assert_eq!(commands.lock().unwrap().len(), MAX_CHANNEL_BUSY_RETRIES + 1);
                    assert!(
                        commands.lock().unwrap().iter()
                            .all(|command| *command == CtapCommand::GetInfo)
                    );
                }
            }
            case "Treats a vendor-specific status as an unknown CTAP error" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#error-responses"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a channel that returns a status byte from the vendor-specific error range" {
                    let vendor_status = 0xe1;
                    let commands = Arc::new(Mutex::new(Vec::new()));
                    let channel = recording_transport(VecDeque::from([vec![vendor_status]]), commands.clone(), Default::default());
                    let mut client = CtapClient::new(Box::new(channel));
                }
                when "the client executes a CTAP request" {
                    let result = client.execute(CtapRequest::new(CtapCommand::GetInfo, Vec::new())).await;
                }
                then "the vendor-specific value is reported through the ordinary unknown-status path" {
                    assert!(matches!(
                        result,
                        Err(CtapClientError::Status(CtapStatusCode::Unknown(0xe1)))
                    ));
                }
            }
    }
    group "CtapClient::get_assertions" {
            case "Get all Assertions from the same Channel" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetNextAssertion"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "Prepare MockCtapTransport, VECDEQUE, and CTAPCLIENT" {
                    let first = assertion(Some(2), 1);
                    let second = assertion(None, 2);
                    let commands = Arc::new(Mutex::new(Vec::new()));
                    let channel = recording_transport(VecDeque::from([
                            success(&first, |value| value.to_cbor_bytes().unwrap()),
                            success(&second, |value| value.to_cbor_bytes().unwrap()),
                        ]), commands.clone(), Default::default());
                    let mut client = CtapClient::new(Box::new(channel));
            }
            when "Call client.get_assertions" {
                let assertions = client.get_assertions(&request(), CtapVersion::Fido21).await.unwrap();
            }
            then "Get all Assertions from the same Channel" {
                assert_eq!(assertions.len(), 2);

                assert_eq!(
                    *commands.lock().unwrap(),
                    vec![CtapCommand::GetAssertion, CtapCommand::GetNextAssertion]
                );
            }

            }
            case "Reject a response with a Credential number in a subsequent Assertion" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetNextAssertion"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "Prepare MockCtapTransport, VECDEQUE, and CTAPCLIENT" {
                    let first = assertion(Some(2), 1);
                    let second = assertion(Some(2), 2);
            }
            when "Call Success" {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let channel = recording_transport(VecDeque::from([
                        success(&first, |value| value.to_cbor_bytes().unwrap()),
                        success(&second, |value| value.to_cbor_bytes().unwrap()),
                    ]), commands.clone(), Default::default());
                let mut client = CtapClient::new(Box::new(channel));
            }
            then "Verify that the results match CtapClientError::NumberOfCredentialsOnNextAssertion" {
                assert!(matches!(
                    client.get_assertions(&request(), CtapVersion::Fido21).await,
                    Err(CtapClientError::NumberOfCredentialsOnNextAssertion)
                ));
            }

            }
            case "Rejects enumeration metadata in an allow-listed assertion response" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "allow-listed GetAssertion requests and responses containing forbidden enumeration metadata" {
                    let descriptor = PublicKeyCredentialDescriptor::new(
                        keyringd_fido::webauthn::PublicKeyCredentialType::PublicKey,
                        vec![1],
                    );
                    let request = request().with_allow_list(vec![descriptor]);
                    let count_response = assertion(Some(2), 1);
                    let selected_response = assertion(None, 1).with_user_selected(true);
                }
                when "each response is returned for an allow-listed request" {
                    let commands = Arc::new(Mutex::new(Vec::new()));
                    let mut count_client = CtapClient::new(Box::new(recording_transport(VecDeque::from([success(&count_response, |value| {
                            value.to_cbor_bytes().unwrap()
                        })]), commands.clone(), Default::default())));
                    let count_result = count_client.get_assertion(&request, CtapVersion::Fido21).await;
                    let commands = Arc::new(Mutex::new(Vec::new()));
                    let mut selected_client = CtapClient::new(Box::new(recording_transport(VecDeque::from([success(&selected_response, |value| {
                            value.to_cbor_bytes().unwrap()
                        })]), commands.clone(), Default::default())));
                    let selected_result = selected_client.get_assertion(&request, CtapVersion::Fido21).await;
                }
                then "the client rejects both fields because allowList permits only one selected credential" {
                    assert!(matches!(
                        count_result,
                        Err(CtapClientError::NumberOfCredentialsWithAllowList)
                    ));
                    assert!(matches!(
                        selected_result,
                        Err(CtapClientError::UserSelectedWithAllowList)
                    ));
                }
            }
            case "Rejects userSelected in a GetNextAssertion response" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetNextAssertion"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a GetNextAssertion response that incorrectly reports direct user selection" {
                    let response = assertion(None, 2).with_user_selected(true);
                    let commands = Arc::new(Mutex::new(Vec::new()));
                    let mut client = CtapClient::new(Box::new(recording_transport(VecDeque::from([success(&response, |value| {
                            value.to_cbor_bytes().unwrap()
                        })]), commands.clone(), Default::default())));
                }
                when "the next assertion is requested" {
                    let result = client.get_next_assertion().await;
                }
                then "the stateful response is rejected because userSelected must be omitted" {
                    assert!(matches!(
                        result,
                        Err(CtapClientError::UserSelectedOnNextAssertion)
                    ));
                }
            }
    }
}

mod pin_uv_auth;
