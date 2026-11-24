use crate::codec::CborCodec;
use crate::codec::CborItem as Value;
use crate::ctap::AuthenticationExtensionsAuthenticatorInputs;
use crate::ctap::AuthenticatorGetAssertionRequest;
use crate::ctap::ClientDataHash;
use crate::ctap::CtapValue;
use crate::ctap::ProtocolError;
use crate::ctap::PublicKeyCredentialDescriptor;
use std::collections::BTreeMap;

keyringd_testkit::scenario! {
    group "AuthenticatorGetAssertionRequest::to_cbor_bytes" {
            case "Encodes every GetAssertion request member under its assigned integer key" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"

                given "a GetAssertion request containing every request member defined by CTAP" {
                    let request = AuthenticatorGetAssertionRequest {
                        rp_id: "example.com".to_owned(),
                        client_data_hash: ClientDataHash::new([0x5a; 32]),
                        allow_list: Some(vec![PublicKeyCredentialDescriptor::new(
                            crate::webauthn::PublicKeyCredentialType::PublicKey,
                            vec![1],
                        )]),
                        extensions: Some(AuthenticationExtensionsAuthenticatorInputs::default()
                            .with_value(
                                "credBlob",
                                crate::EncodedCborValue::from_canonical_bytes(vec![0xf5]).unwrap(),
                            )),
                        options: Some(BTreeMap::from([("up".to_owned(), true)])),
                        pin_uv_auth_param: Some(vec![2; 16]),
                        pin_uv_auth_protocol: Some(2),
                    };
                }
                when "the request is encoded as a canonical CTAP CBOR map" {
                    let encoded = request.to_cbor_bytes().unwrap();
                    let fields = CtapValue::ctap_map(CborCodec::decode(&encoded).unwrap())
                        .unwrap();
                }
                then "every key contains the value and CBOR type assigned by authenticatorGetAssertion" {
                    let canonical = |value| {
                        CborCodec::decode(&CborCodec::encode(value).unwrap()).unwrap()
                    };
                    assert_eq!(fields.keys().copied().collect::<Vec<_>>(), (1..=7).collect::<Vec<_>>());
                    assert_eq!(fields[&1], Value::TextString("example.com".to_owned()));
                    assert_eq!(fields[&2], Value::ByteString(vec![0x5a; 32]));
                    assert_eq!(
                        fields[&3],
                        canonical(PublicKeyCredentialDescriptor::vec_to_cbor_value(
                            request.allow_list.as_ref().unwrap(),
                        ).unwrap())
                    );
                    assert_eq!(
                        fields[&4],
                        canonical(request.extensions.as_ref().unwrap().to_cbor_value().unwrap())
                    );
                    assert_eq!(
                        fields[&5],
                        Value::Map(vec![(
                            Value::TextString("up".to_owned()),
                            Value::bool_value(true),
                        )])
                    );
                    assert_eq!(fields[&6], Value::ByteString(vec![2; 16]));
                    assert_eq!(fields[&7], Value::Unsigned(2));
                    assert_eq!(
                        AuthenticatorGetAssertionRequest::from_cbor_bytes(&encoded).unwrap(),
                        request
                    );
                }
            }
            case "Omits an empty allow list supplied through the platform builder" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getassert-allowlist"

                given "a valid GetAssertion request whose allowed credential candidates are empty" {
                    let request = AuthenticatorGetAssertionRequest::new(
                        "example.com",
                        ClientDataHash::new([0; 32]),
                    )
                    .with_allow_list(Vec::new());
                }
                when "the request is encoded and decoded through the CTAP wire representation" {
                    let decoded = AuthenticatorGetAssertionRequest::from_cbor_bytes(
                        &request.to_cbor_bytes().unwrap(),
                    )
                    .unwrap();
                }
                then "allowList is absent rather than present as an empty array" {
                    assert_eq!(request.allow_list(), None);
                    assert_eq!(decoded.allow_list(), None);
                }
            }
            case "Round-trips a discoverable credential request" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"

                given "Prepare a Discoverable Credential request that omits the ALLOW LIST" {
                    let request = AuthenticatorGetAssertionRequest {
                        rp_id: "example.com".to_owned(),
                        client_data_hash: ClientDataHash::new([0x5a; 32]),
                        allow_list: None,
                        extensions: None,
                        options: Some(BTreeMap::from([("up".to_owned(), true)])),
                        pin_uv_auth_param: None,
                        pin_uv_auth_protocol: None,
                    };
                }
                when "Encode requests to CBOR and then decode" {
                    let decoded = AuthenticatorGetAssertionRequest::from_cbor_bytes(
                        &request.to_cbor_bytes().unwrap(),
                    )
                    .unwrap();
                }
                then "The original GetAssertion request is restored" {
                    assert_eq!(
                        decoded,
                        request
                    );
                }

            }
            case "Rejects empty permission list" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"

                given "Prepare a GetAssertion request that explicitly states an empty Allow List" {
                    let request = AuthenticatorGetAssertionRequest {
                        rp_id: "example.com".to_owned(),
                        client_data_hash: ClientDataHash::new([0; 32]),
                        allow_list: Some(Vec::new()),
                        extensions: None,
                        options: None,
                        pin_uv_auth_param: None,
                        pin_uv_auth_protocol: None,
                    };
                }
                when "Encode the request to CBOR" {
                    let result = request.to_cbor_bytes();
                }
                then "AllowList is rejected as empty" {
                    assert!(matches!(result, Err(ProtocolError::EmptyValue { field: "allowList" })));
                }

            }
            case "Can represent protocol specifications without authentication parameters" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"

                given "Prepare AuthenticatorGetAssertionRequest/ClientDataHash" {
                    let request = AuthenticatorGetAssertionRequest {
                        rp_id: "example.com".to_owned(),
                        client_data_hash: ClientDataHash::new([0; 32]),
                        allow_list: None,
                        extensions: None,
                        options: None,
                        pin_uv_auth_param: None,
                        pin_uv_auth_protocol: Some(1),
                    };
                }
                when "Call request.to_cbor_bytes" {
                    let encoded = request.to_cbor_bytes().unwrap();
                }
                then "A PROTOCOL specification and an unspecified authentication parameter are retained even after the decode" {
                    assert_eq!(
                        AuthenticatorGetAssertionRequest::from_cbor_bytes(&encoded).unwrap(),
                        request
                    );
                }

            }
            case "Rejects GetAssertion options forbidden by the platform requirements" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getassert-rk"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getassert-uv"

                given "a valid GetAssertion request template" {
                    let request = AuthenticatorGetAssertionRequest::new(
                        "example.com",
                        ClientDataHash::new([0; 32]),
                    );
                }
                when "the platform adds rk or combines the uv option with pinUvAuthParam" {
                    let rk = request.clone().with_options(BTreeMap::from([
                        ("rk".to_owned(), false),
                    ]));
                    let duplicate_uv = request
                        .with_options(BTreeMap::from([("uv".to_owned(), false)]))
                        .with_pin_uv_auth(vec![0; 16], 2);
                }
                then "both requests are rejected before they can be sent to an authenticator" {
                    assert!(matches!(rk.to_cbor_bytes(), Err(ProtocolError::InvalidValue {
                            field: "options.rk",
                            expected: "absent from GetAssertion requests",
                        })));
                    assert!(matches!(duplicate_uv.to_cbor_bytes(), Err(ProtocolError::InvalidValue {
                            field: "options.uv/pinUvAuthParam",
                            expected: "at most one to be present",
                        })));
                }
            }
    }
}
