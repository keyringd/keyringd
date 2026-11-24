use crate::codec::CborCodec;
use crate::codec::CborItem as Value;
use crate::ctap::AuthenticationExtensionsAuthenticatorInputs;
use crate::ctap::AuthenticatorMakeCredentialRequest;
use crate::ctap::ClientDataHash;
use crate::ctap::CtapValue;
use crate::ctap::ProtocolError;
use crate::ctap::PublicKeyCredentialDescriptor;
use crate::ctap::PublicKeyCredentialParameters;
use crate::ctap::PublicKeyCredentialRpEntity;
use crate::ctap::PublicKeyCredentialUserEntity;
use std::collections::BTreeMap;

keyringd_testkit::scenario! {
    group "AuthenticatorMakeCredentialRequest::to_cbor_bytes" {
            case "Encodes every MakeCredential request member under its assigned integer key" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorMakeCredential"

                given "a MakeCredential request containing every request member defined by CTAP" {
                    let request = AuthenticatorMakeCredentialRequest {
                        client_data_hash: ClientDataHash::new([0x42; 32]),
                        relying_party: PublicKeyCredentialRpEntity::from_rp_id("example.com"),
                        user: PublicKeyCredentialUserEntity::from_id(vec![1]),
                        public_key_credential_parameters: vec![PublicKeyCredentialParameters::es256()],
                        exclude_list: Some(vec![PublicKeyCredentialDescriptor::new(
                            crate::webauthn::PublicKeyCredentialType::PublicKey,
                            vec![2],
                        )]),
                        extensions: Some(AuthenticationExtensionsAuthenticatorInputs::default()
                            .with_value(
                                "credProps",
                                crate::EncodedCborValue::from_canonical_bytes(vec![0xf5]).unwrap(),
                            )),
                        options: Some(BTreeMap::from([("rk".to_owned(), true)])),
                        pin_uv_auth_param: Some(vec![3; 16]),
                        pin_uv_auth_protocol: Some(2),
                        enterprise_attestation: Some(1),
                        attestation_formats_preference: Some(vec!["none".to_owned()]),
                    };
                }
                when "the request is encoded as a canonical CTAP CBOR map" {
                    let encoded = request.to_cbor_bytes().unwrap();
                    let fields = CtapValue::ctap_map(CborCodec::decode(&encoded).unwrap())
                        .unwrap();
                }
                then "every key contains the value and CBOR type assigned by authenticatorMakeCredential" {
                    let canonical = |value| {
                        CborCodec::decode(&CborCodec::encode(value).unwrap()).unwrap()
                    };
                    assert_eq!(fields.keys().copied().collect::<Vec<_>>(), (1..=11).collect::<Vec<_>>());
                    assert_eq!(fields[&1], Value::ByteString(vec![0x42; 32]));
                    assert_eq!(fields[&2], canonical(request.relying_party.to_cbor_value().unwrap()));
                    assert_eq!(fields[&3], canonical(request.user.to_cbor_value().unwrap()));
                    assert_eq!(
                        fields[&4],
                        canonical(PublicKeyCredentialParameters::vec_to_value(
                            &request.public_key_credential_parameters,
                        )
                        .unwrap())
                    );
                    assert_eq!(
                        fields[&5],
                        canonical(PublicKeyCredentialDescriptor::vec_to_cbor_value(
                            request.exclude_list.as_ref().unwrap(),
                        )
                        .unwrap())
                    );
                    assert_eq!(
                        fields[&6],
                        canonical(request.extensions.as_ref().unwrap().to_cbor_value().unwrap())
                    );
                    assert_eq!(
                        fields[&7],
                        Value::Map(vec![(
                            Value::TextString("rk".to_owned()),
                            Value::bool_value(true),
                        )])
                    );
                    assert_eq!(fields[&8], Value::ByteString(vec![3; 16]));
                    assert_eq!(fields[&9], Value::Unsigned(2));
                    assert_eq!(fields[&10], Value::Unsigned(1));
                    assert_eq!(
                        fields[&11],
                        Value::Array(vec![Value::TextString("none".to_owned())])
                    );
                    assert_eq!(
                        AuthenticatorMakeCredentialRequest::from_cbor_bytes(&encoded).unwrap(),
                        request
                    );
                }
            }
            case "Preserves public-key credential parameter preference order" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#makecred-pubkeycredparams"

                given "a MakeCredential request that prefers ES256 over a second registered-style algorithm identifier" {
                    let preferred = PublicKeyCredentialParameters::es256();
                    let fallback = PublicKeyCredentialParameters::new(
                        crate::webauthn::PublicKeyCredentialType::PublicKey,
                        crate::webauthn::CoseAlgorithmIdentifier::new(-257),
                    );
                    let request = AuthenticatorMakeCredentialRequest::new(
                        ClientDataHash::new([0; 32]),
                        PublicKeyCredentialRpEntity::from_rp_id("example.com"),
                        PublicKeyCredentialUserEntity::from_id(vec![1]),
                        vec![preferred.clone(), fallback.clone()],
                    );
                }
                when "the request is encoded and decoded through the CTAP wire representation" {
                    let decoded = AuthenticatorMakeCredentialRequest::from_cbor_bytes(
                        &request.to_cbor_bytes().unwrap(),
                    )
                    .unwrap();
                }
                then "the most-preferred-to-least-preferred order is unchanged" {
                    assert_eq!(
                        decoded.public_key_credential_parameters(),
                        &[preferred, fallback]
                    );
                }
            }
            case "Preserves attestation format preference order and the single none request" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#makecred-attestationformatspreference"

                given "prioritized attestation formats and a separate request for no attestation" {
                    let request = || AuthenticatorMakeCredentialRequest::new(
                        ClientDataHash::new([0; 32]),
                        PublicKeyCredentialRpEntity::from_rp_id("example.com"),
                        PublicKeyCredentialUserEntity::from_id(vec![1]),
                        vec![PublicKeyCredentialParameters::es256()],
                    );
                    let prioritized = request().with_attestation_formats_preference(vec![
                        "packed".to_owned(),
                        "fido-u2f".to_owned(),
                    ]);
                    let none = request()
                        .with_attestation_formats_preference(vec!["none".to_owned()]);
                }
                when "both requests are encoded for the authenticator" {
                    let prioritized_fields = CtapValue::ctap_map(
                        CborCodec::decode(&prioritized.to_cbor_bytes().unwrap()).unwrap(),
                    )
                    .unwrap();
                    let none_fields = CtapValue::ctap_map(
                        CborCodec::decode(&none.to_cbor_bytes().unwrap()).unwrap(),
                    )
                    .unwrap();
                }
                then "key eleven preserves priority and represents omission as the single string none" {
                    assert_eq!(
                        prioritized_fields[&11],
                        Value::Array(vec![
                            Value::TextString("packed".to_owned()),
                            Value::TextString("fido-u2f".to_owned()),
                        ])
                    );
                    assert_eq!(
                        none_fields[&11],
                        Value::Array(vec![Value::TextString("none".to_owned())])
                    );
                }
            }
            case "Omits an empty exclude list supplied through the platform builder" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#makecred-excludelist"

                given "a valid MakeCredential request whose exclusion candidates are empty" {
                    let request = AuthenticatorMakeCredentialRequest::new(
                        ClientDataHash::new([0; 32]),
                        PublicKeyCredentialRpEntity::from_rp_id("example.com"),
                        PublicKeyCredentialUserEntity::from_id(vec![1]),
                        vec![PublicKeyCredentialParameters::es256()],
                    )
                    .with_exclude_list(Vec::new());
                }
                when "the request is encoded and decoded through the CTAP wire representation" {
                    let decoded = AuthenticatorMakeCredentialRequest::from_cbor_bytes(
                        &request.to_cbor_bytes().unwrap(),
                    )
                    .unwrap();
                }
                then "excludeList is absent rather than present as an empty array" {
                    assert_eq!(request.exclude_list(), None);
                    assert_eq!(decoded.exclude_list(), None);
                }
            }
            case "Round-trips an ES256 discoverable credential request" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorMakeCredential"

                given "Prepare a MakeCredential request for ES256 requesting Discoverable Credential" {
                    let request = AuthenticatorMakeCredentialRequest {
                        client_data_hash: ClientDataHash::new([0x42; 32]),
                        relying_party: PublicKeyCredentialRpEntity::new("Example").with_id("example.com"),
                        user: PublicKeyCredentialUserEntity::new(
                            "user@example.com",
                            vec![1, 2, 3],
                            "Example User",
                        ),
                        public_key_credential_parameters: vec![PublicKeyCredentialParameters::es256()],
                        exclude_list: None,
                        extensions: None,
                        options: Some(BTreeMap::from([("rk".to_owned(), true)])),
                        pin_uv_auth_param: None,
                        pin_uv_auth_protocol: None,
                        enterprise_attestation: None,
                        attestation_formats_preference: Some(vec!["none".to_owned()]),
                    };
                }
                when "Encode requests to CBOR and then decode" {
                    let decoded = AuthenticatorMakeCredentialRequest::from_cbor_bytes(
                        &request.to_cbor_bytes().unwrap(),
                    )
                    .unwrap();
                }
                then "The original MakeCredential request is restored" {
                    assert_eq!(
                        decoded,
                        request
                    );
                }

            }
            case "Can represent protocol specifications without authentication parameters" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorMakeCredential"

                given "Prepare AuthenticatorMakeCredentialRequest, ClientDataHash, and PublicKeyCredentialRpEntity" {
                    let mut request = AuthenticatorMakeCredentialRequest {
                        client_data_hash: ClientDataHash::new([0; 32]),
                        relying_party: PublicKeyCredentialRpEntity::from_rp_id("example.com"),
                        user: PublicKeyCredentialUserEntity::from_id(Vec::new()),
                        public_key_credential_parameters: vec![PublicKeyCredentialParameters::es256()],
                        exclude_list: None,
                        extensions: None,
                        options: None,
                        pin_uv_auth_param: None,
                        pin_uv_auth_protocol: Some(2),
                        enterprise_attestation: None,
                        attestation_formats_preference: None,
                    };
                }
                when "Call request.to_cbor_bytes" {
                    let encoded = request.to_cbor_bytes().unwrap();
                }
                then "Can represent protocol specifications without authentication parameters" {
                    assert_eq!(
                        AuthenticatorMakeCredentialRequest::from_cbor_bytes(&encoded).unwrap(),
                        request
                    );
                    request.pin_uv_auth_param = Some(Vec::new());
                    request.pin_uv_auth_protocol = None;
                    assert!(request.to_cbor_bytes().is_err());
                }

            }
            case "Duplicate public key Credential parameter is rejected when encoding" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorMakeCredential"

                given "Prepare AuthenticatorMakeCredentialRequest, ClientDataHash, and PublicKeyCredentialRpEntity" {
                    let mut request = AuthenticatorMakeCredentialRequest {
                        client_data_hash: ClientDataHash::new([0; 32]),
                        relying_party: PublicKeyCredentialRpEntity::from_rp_id("example.com"),
                        user: PublicKeyCredentialUserEntity::from_id(vec![1]),
                        public_key_credential_parameters: vec![PublicKeyCredentialParameters::es256()],
                        exclude_list: None,
                        extensions: None,
                        options: None,
                        pin_uv_auth_param: None,
                        pin_uv_auth_protocol: None,
                        enterprise_attestation: None,
                        attestation_formats_preference: None,
                    };
                }
                when "Call PublicKeyCredentialParameters::es256" {
                    request
                        .public_key_credential_parameters
                        .push(PublicKeyCredentialParameters::es256());
                }
                then "Verify that the results match ProtocolError::DuplicateValue" {
                    assert!(matches!(request.to_cbor_bytes(), Err(ProtocolError::DuplicateValue {
                            field: "pubKeyCredParams"
                        })));
                }

            }
            case "Rejects MakeCredential options forbidden by the platform requirements" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#makecred-up"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#makecred-uv"

                given "a valid MakeCredential request template" {
                    let request = AuthenticatorMakeCredentialRequest::new(
                        ClientDataHash::new([0; 32]),
                        PublicKeyCredentialRpEntity::from_rp_id("example.com"),
                        PublicKeyCredentialUserEntity::from_id(vec![1]),
                        vec![PublicKeyCredentialParameters::es256()],
                    );
                }
                when "the platform requests up=false or combines the uv option with pinUvAuthParam" {
                    let false_up = request.clone().with_options(BTreeMap::from([
                        ("up".to_owned(), false),
                    ]));
                    let duplicate_uv = request
                        .with_options(BTreeMap::from([("uv".to_owned(), false)]))
                        .with_pin_uv_auth(vec![0; 16], 2);
                }
                then "both requests are rejected before they can be sent to an authenticator" {
                    assert!(matches!(false_up.to_cbor_bytes(), Err(ProtocolError::InvalidValue {
                            field: "options.up",
                            expected: "true when present",
                        })));
                    assert!(matches!(duplicate_uv.to_cbor_bytes(), Err(ProtocolError::InvalidValue {
                            field: "options.uv/pinUvAuthParam",
                            expected: "at most one to be present",
                        })));
                }
            }
            case "Rejects undefined enterprise attestation request values" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#op-makecred-step-enterprise-attestation"

                given "a valid MakeCredential request carrying enterpriseAttestation value three" {
                    let request = AuthenticatorMakeCredentialRequest::new(
                        ClientDataHash::new([0; 32]),
                        PublicKeyCredentialRpEntity::from_rp_id("example.com"),
                        PublicKeyCredentialUserEntity::from_id(vec![1]),
                        vec![PublicKeyCredentialParameters::es256()],
                    )
                    .with_enterprise_attestation(3);
                }
                when "the request is encoded for CTAP" {
                    let result = request.to_cbor_bytes();
                }
                then "only the specification-defined values one and two are accepted" {
                    assert!(matches!(result, Err(ProtocolError::InvalidValue {
                            field: "enterpriseAttestation",
                            expected: "1 or 2",
                        })));
                }
            }
    }
}
