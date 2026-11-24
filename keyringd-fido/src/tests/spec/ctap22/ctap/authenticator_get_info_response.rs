use crate::AuthenticatorTransport;
use crate::codec::CborCodec;
use crate::codec::CborItem as Value;
use crate::ctap::AuthenticatorGetInfoResponse;
use crate::ctap::CtapValue;
use crate::ctap::ProtocolError;
use crate::ctap::PublicKeyCredentialParameters;
use crate::tests::support::ctap_authenticator_get_info_response::minimal;
use std::collections::BTreeMap;

keyringd_testkit::scenario! {
    group "AuthenticatorGetInfoResponse::from_cbor_bytes" {
            case "Round-trips every field defined by authenticatorGetInfo" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

                given "a valid GetInfo response containing fields 0x01 through 0x1d" {
                    let response = AuthenticatorGetInfoResponse::new(
                        vec!["FIDO_2_1".to_owned()],
                        [0x11; 16],
                    )
                    .with_extensions(vec![
                        "credBlob".to_owned(),
                        "credProtect".to_owned(),
                        "hmac-secret".to_owned(),
                        "pinComplexityPolicy".to_owned(),
                    ])
                    .with_options(BTreeMap::from([
                        ("clientPin".to_owned(), true),
                        ("largeBlobs".to_owned(), true),
                        ("pinUvAuthToken".to_owned(), true),
                        ("rk".to_owned(), true),
                        ("setMinPINLength".to_owned(), true),
                        ("uv".to_owned(), true),
                    ]))
                    .with_max_message_size(4096)
                    .with_pin_uv_auth_protocols(vec![2, 1])
                    .with_max_credential_count_in_list(8)
                    .with_max_credential_id_length(1024)
                    .with_transports(vec![
                        AuthenticatorTransport::usb(),
                        AuthenticatorTransport::smart_card(),
                    ])
                    .with_algorithms(vec![PublicKeyCredentialParameters::es256()])
                    .with_max_serialized_large_blob_array(2048)
                    .with_force_pin_change(false)
                    .with_min_pin_length(4)
                    .with_firmware_version(0x0102_0304)
                    .with_max_credential_blob_length(32)
                    .with_max_rp_ids_for_set_min_pin_length(8)
                    .with_preferred_platform_uv_attempts(3)
                    .with_uv_modality(1)
                    .with_certifications(BTreeMap::from([("FIDO".to_owned(), 2)]))
                    .with_remaining_discoverable_credentials(7)
                    .with_vendor_prototype_config_commands(vec![0x40])
                    .with_attestation_formats(vec!["packed".to_owned()])
                    .with_uv_count_since_last_pin_entry(0)
                    .with_long_touch_for_reset(true)
                    .with_encrypted_identifier(vec![0x22; 32])
                    .with_transports_for_reset(vec![AuthenticatorTransport::usb()])
                    .with_pin_complexity_policy(true)
                    .with_pin_complexity_policy_url(b"https://example.com/pin-policy".to_vec())
                    .with_max_pin_length(63);
                }
                when "the complete response is encoded and decoded" {
                    let encoded = response.to_cbor_bytes().unwrap();
                    let decoded = AuthenticatorGetInfoResponse::from_cbor_bytes(&encoded).unwrap();
                    let fields = CtapValue::ctap_map(
                        CborCodec::decode(&encoded).unwrap(),
                    )
                    .unwrap();
                }
                then "every field retains its value and occupies its assigned integer key" {
                    assert_eq!(decoded, response);
                    let canonical = |value| {
                        CborCodec::decode(&CborCodec::encode(value).unwrap()).unwrap()
                    };
                    assert_eq!(
                        fields,
                        BTreeMap::from([
                            (1, Value::Array(vec![
                                Value::TextString("FIDO_2_1".to_owned()),
                            ])),
                            (2, Value::Array(vec![
                                Value::TextString("credBlob".to_owned()),
                                Value::TextString("credProtect".to_owned()),
                                Value::TextString("hmac-secret".to_owned()),
                                Value::TextString("pinComplexityPolicy".to_owned()),
                            ])),
                            (3, Value::ByteString(vec![0x11; 16])),
                            (4, canonical(Value::Map(vec![
                                (Value::TextString("clientPin".to_owned()), Value::bool_value(true)),
                                (Value::TextString("largeBlobs".to_owned()), Value::bool_value(true)),
                                (Value::TextString("pinUvAuthToken".to_owned()), Value::bool_value(true)),
                                (Value::TextString("rk".to_owned()), Value::bool_value(true)),
                                (Value::TextString("setMinPINLength".to_owned()), Value::bool_value(true)),
                                (Value::TextString("uv".to_owned()), Value::bool_value(true)),
                            ]))),
                            (5, Value::Unsigned(4096)),
                            (6, Value::Array(vec![Value::Unsigned(2), Value::Unsigned(1)])),
                            (7, Value::Unsigned(8)),
                            (8, Value::Unsigned(1024)),
                            (9, Value::Array(vec![
                                Value::TextString("usb".to_owned()),
                                Value::TextString("smart-card".to_owned()),
                            ])),
                            (10, canonical(PublicKeyCredentialParameters::vec_to_value(
                                &[PublicKeyCredentialParameters::es256()],
                            ).unwrap())),
                            (11, Value::Unsigned(2048)),
                            (12, Value::bool_value(false)),
                            (13, Value::Unsigned(4)),
                            (14, Value::Unsigned(0x0102_0304)),
                            (15, Value::Unsigned(32)),
                            (16, Value::Unsigned(8)),
                            (17, Value::Unsigned(3)),
                            (18, Value::Unsigned(1)),
                            (19, Value::Map(vec![(
                                Value::TextString("FIDO".to_owned()),
                                Value::Unsigned(2),
                            )])),
                            (20, Value::Unsigned(7)),
                            (21, Value::Array(vec![Value::Unsigned(0x40)])),
                            (22, Value::Array(vec![Value::TextString("packed".to_owned())])),
                            (23, Value::Unsigned(0)),
                            (24, Value::bool_value(true)),
                            (25, Value::ByteString(vec![0x22; 32])),
                            (26, Value::Array(vec![Value::TextString("usb".to_owned())])),
                            (27, Value::bool_value(true)),
                            (28, Value::ByteString(b"https://example.com/pin-policy".to_vec())),
                            (29, Value::Unsigned(63)),
                        ])
                    );
                }
            }
            case "Preserves unknown transport and attestation-format identifiers" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-transports"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-attestationformats"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-transportsforreset"

                given "a GetInfo response using identifiers introduced by a future specification" {
                    let response = minimal()
                        .with_transports(vec![AuthenticatorTransport::new("future-transport")])
                        .with_attestation_formats(vec!["future-attestation".to_owned()])
                        .with_transports_for_reset(vec![AuthenticatorTransport::new(
                            "future-reset-transport",
                        )]);
                }
                when "the response is encoded and decoded by the current implementation" {
                    let decoded = AuthenticatorGetInfoResponse::from_cbor_bytes(
                        &response.to_cbor_bytes().unwrap(),
                    )
                    .unwrap();
                }
                then "all unknown identifiers remain intact for forward compatibility" {
                    assert_eq!(
                        decoded.transports().unwrap()[0].as_str(),
                        "future-transport"
                    );
                    assert_eq!(
                        decoded.attestation_formats(),
                        Some(["future-attestation".to_owned()].as_slice())
                    );
                    assert_eq!(
                        decoded.transports_for_reset().unwrap()[0].as_str(),
                        "future-reset-transport"
                    );
                }
            }
            case "Ignores an unknown GetInfo field whose value is floating point" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#message-encoding"

                given "a canonical GetInfo response with required fields and an unknown key containing a 16-bit floating-point value" {
                    let mut encoded = vec![
                        0xa3,
                        0x01, 0x81, 0x68, b'F', b'I', b'D', b'O', b'_', b'2', b'_', b'1',
                        0x03, 0x50,
                    ];
                    encoded.extend_from_slice(&[0; 16]);
                    encoded.extend_from_slice(&[0x18, 0x20, 0xf9, 0x3e, 0x00]);
                }
                when "the response is decoded by a client that does not understand the additional key" {
                    let response = AuthenticatorGetInfoResponse::from_cbor_bytes(&encoded).unwrap();
                }
                then "the unknown key is ignored and the understood fields remain available" {
                    assert_eq!(response.versions(), ["FIDO_2_1"]);
                    assert_eq!(response.aaguid(), &[0; 16]);
                }
            }
            case "GetInfo response can be converted back and forth" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

                given "Prepare a Response" {
                    let response = minimal();
                }
                when "Call Response.to_cbor_bytes" {
                    let bytes = response.to_cbor_bytes().unwrap();
                }
                then "The GetInfo response after decode matches the pre-Encode" {
                    assert_eq!(
                        AuthenticatorGetInfoResponse::from_cbor_bytes(&bytes).unwrap(),
                        response
                    );
                }

            }
            case "Reject Missing of Mandatory Fields" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

                given "Prepare a GetInfo Response CBOR with a Missing AAGUID Field" {
                    let bytes = hex_literal::hex!(
                        "a1
                         01 81
                            68 4649444f5f325f31"
                    );
                }
                when "Decode as a getInfo response" {
                    let result = AuthenticatorGetInfoResponse::from_cbor_bytes(&bytes);
                }
                then "Rejected as missing AAGUID FIELD" {
                    assert!(matches!(result, Err(ProtocolError::MissingCtapField(0x03))));
                }

            }
            case "Rejects an AAGUID whose byte string is not 16 bytes" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

                given "a GetInfo response containing a 15-byte AAGUID" {
                    let encoded = CborCodec::encode(Value::Map(vec![
                        (1u64.into(), Value::Array(vec![Value::TextString("FIDO_2_1".to_owned())])),
                        (3u64.into(), Value::ByteString(vec![0; 15])),
                    ])).unwrap();
                }
                when "the response is decoded" {
                    let result = AuthenticatorGetInfoResponse::from_cbor_bytes(&encoded);
                }
                then "the exact AAGUID length is enforced" {
                    assert!(matches!(result, Err(ProtocolError::InvalidLength {
                            field: "aaguid",
                            expected: 16,
                            actual: 15,
                        })));
                }
            }
    }
    group "AuthenticatorGetInfoResponse::to_cbor_bytes" {
            case "Rejects an empty PIN/UV Auth Protocol preference list" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-pinuvauthprotocols"

                given "a GetInfo response whose pinUvAuthProtocols field is present but empty" {
                    let response = minimal().with_pin_uv_auth_protocols(Vec::new());
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "encoding rejects the empty preference list" {
                    assert!(result.is_err());
                }
            }
            case "Rejects duplicate PIN/UV Auth Protocol identifiers" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-pinuvauthprotocols"

                given "a GetInfo response that advertises PIN/UV Auth Protocol 2 twice" {
                    let response = minimal().with_pin_uv_auth_protocols(vec![2, 2]);
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "encoding rejects the duplicate protocol identifier" {
                    assert!(result.is_err());
                }
            }
            case "Rejects duplicate authenticator transports" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-transports"

                given "a GetInfo response that advertises the USB transport twice" {
                    let response = minimal().with_transports(vec![
                        AuthenticatorTransport::usb(),
                        AuthenticatorTransport::usb(),
                    ]);
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "encoding rejects the duplicate transport" {
                    assert!(result.is_err());
                }
            }
            case "Rejects an empty authenticator transport list" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-transports"

                given "a GetInfo response whose transports field is present but empty" {
                    let response = minimal().with_transports(Vec::new());
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "encoding rejects the empty transport list" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::EmptyValue { field: "transports" })
                    ));
                }
            }
            case "Rejects duplicate public-key credential algorithms" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

                given "a GetInfo response that advertises the same credential type and algorithm twice" {
                    let response = minimal().with_algorithms(vec![
                        PublicKeyCredentialParameters::es256(),
                        PublicKeyCredentialParameters::es256(),
                    ]);
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "encoding rejects the duplicate algorithm entry" {
                    assert!(result.is_err());
                }
            }
            case "Rejects an empty public-key credential algorithm list" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

                given "a GetInfo response whose algorithms field is present but empty" {
                    let response = minimal().with_algorithms(Vec::new());
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "encoding rejects the empty algorithm preference list" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::EmptyValue { field: "algorithms" })
                    ));
                }
            }
            case "Rejects an empty attestation-format list" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-attestationformats"

                given "a GetInfo response whose attestationFormats field is present but empty" {
                    let response = minimal().with_attestation_formats(Vec::new());
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "encoding rejects the empty format list" {
                    assert!(result.is_err());
                }
            }
            case "Rejects duplicate attestation formats" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-attestationformats"

                given "a GetInfo response that advertises the packed attestation format twice" {
                    let response = minimal()
                        .with_attestation_formats(vec!["packed".to_owned(), "packed".to_owned()]);
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "encoding rejects the duplicate attestation-format identifier" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::DuplicateValue {
                            field: "attestationFormats"
                        })
                    ));
                }
            }
            case "Rejects the implicitly supported none attestation format" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-attestationformats"

                given "a GetInfo response that explicitly advertises the none attestation format" {
                    let response = minimal().with_attestation_formats(vec!["none".to_owned()]);
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "encoding rejects the format that the specification requires to be omitted" {
                    assert!(result.is_err());
                }
            }
            case "Rejects a zero maximum credential-list count" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

                given "a GetInfo response with maxCredentialCountInList set to zero" {
                    let response = minimal().with_max_credential_count_in_list(0);
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "encoding rejects the non-positive credential-list limit" {
                    assert!(result.is_err());
                }
            }
            case "Rejects a zero maximum credential-identifier length" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

                given "a GetInfo response with maxCredentialIdLength set to zero" {
                    let response = minimal().with_max_credential_id_length(0);
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "encoding rejects the non-positive credential-identifier limit" {
                    assert!(result.is_err());
                }
            }
            case "Rejects duplicate transports for authenticator reset" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-transportsforreset"

                given "a GetInfo response that advertises USB twice in transportsForReset" {
                    let response = minimal().with_transports_for_reset(vec![
                        AuthenticatorTransport::usb(),
                        AuthenticatorTransport::usb(),
                    ]);
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "encoding rejects the duplicate reset transport" {
                    assert!(result.is_err());
                }
            }
            case "Rejects an empty reset-transport list" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-transportsforreset"

                given "a GetInfo response whose transportsForReset field is present but empty" {
                    let response = minimal().with_transports_for_reset(Vec::new());
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "encoding rejects the empty reset-transport list" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::EmptyValue {
                            field: "transportsForReset"
                        })
                    ));
                }
            }
            case "Rejects an encrypted authenticator identifier with the wrong length" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-encidentifier"

                given "a GetInfo response with a 31-byte encrypted authenticator identifier" {
                    let response = minimal().with_encrypted_identifier(vec![0; 31]);
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "encoding rejects the identifier because iv and ciphertext require 32 bytes" {
                    assert!(result.is_err());
                }
            }
            case "Preserves authenticator preference order for valid capability arrays" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

                given "a GetInfo response with ordered protocol and algorithm preferences" {
                    let response = minimal()
                        .with_pin_uv_auth_protocols(vec![2, 1])
                        .with_algorithms(vec![
                            PublicKeyCredentialParameters::es256(),
                            PublicKeyCredentialParameters::new(
                                crate::webauthn::PublicKeyCredentialType::PublicKey,
                                crate::webauthn::CoseAlgorithmIdentifier::new(-8),
                            ),
                        ]);
                }
                when "the response is encoded and decoded" {
                    let decoded = AuthenticatorGetInfoResponse::from_cbor_bytes(
                        &response.to_cbor_bytes().unwrap(),
                    )
                    .unwrap();
                }
                then "the decreasing authenticator preference order remains unchanged" {
                    assert_eq!(decoded.pin_uv_auth_protocols(), Some([2, 1].as_slice()));
                    assert_eq!(
                        decoded
                            .algorithms()
                            .unwrap()
                            .iter()
                            .map(PublicKeyCredentialParameters::algorithm)
                            .collect::<Vec<_>>(),
                        vec![
                            crate::webauthn::CoseAlgorithmIdentifier::ES256,
                            crate::webauthn::CoseAlgorithmIdentifier::new(-8),
                        ]
                    );
                }

            }
            case "Rejects cross-field capability contradictions" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

                given "GetInfo responses whose dependent fields contradict the advertised extensions and options" {
                    let missing_min_pin_length = AuthenticatorGetInfoResponse::new(
                        vec!["FIDO_2_1".to_owned()],
                        [0; 16],
                    )
                    .with_options(BTreeMap::from([("clientPin".to_owned(), true)]));
                    let unexpected_max_pin_length = AuthenticatorGetInfoResponse::new(
                        vec!["FIDO_2_1".to_owned()],
                        [0; 16],
                    )
                    .with_max_pin_length(63);
                    let missing_credential_blob_length = AuthenticatorGetInfoResponse::new(
                        vec!["FIDO_2_1".to_owned()],
                        [0; 16],
                    )
                    .with_extensions(vec!["credBlob".to_owned()]);
                    let unexpected_large_blob_size = AuthenticatorGetInfoResponse::new(
                        vec!["FIDO_2_1".to_owned()],
                        [0; 16],
                    )
                    .with_max_serialized_large_blob_array(1024);
                    let missing_max_rp_ids = minimal().with_options(BTreeMap::from([
                        ("clientPin".to_owned(), false),
                        ("setMinPINLength".to_owned(), true),
                    ]));
                }
                when "each contradictory response is encoded" {
                    let results = [
                        missing_min_pin_length.to_cbor_bytes(),
                        unexpected_max_pin_length.to_cbor_bytes(),
                        missing_credential_blob_length.to_cbor_bytes(),
                        unexpected_large_blob_size.to_cbor_bytes(),
                        missing_max_rp_ids.to_cbor_bytes(),
                    ];
                }
                then "each response is rejected at the field whose presence is inconsistent" {
                    assert!(matches!(
                        &results[0],
                        Err(ProtocolError::UnexpectedType { field: "minPINLength", .. })
                    ));
                    assert!(matches!(
                        &results[1],
                        Err(ProtocolError::UnexpectedType { field: "maxPINLength", .. })
                    ));
                    assert!(matches!(
                        &results[2],
                        Err(ProtocolError::UnexpectedType { field: "maxCredBlobLength", .. })
                    ));
                    assert!(matches!(
                        &results[3],
                        Err(ProtocolError::UnexpectedType {
                            field: "maxSerializedLargeBlobArray",
                            ..
                        })
                    ));
                    assert!(matches!(
                        &results[4],
                        Err(ProtocolError::UnexpectedType {
                            field: "maxRPIDsForSetMinPINLength",
                            ..
                        })
                    ));
                }
            }
            case "Rejects a minimum PIN length below the CTAP lower bound" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

                given "a GetInfo response advertising client PIN support with minPINLength set to three" {
                    let response = minimal().with_min_pin_length(3);
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "the response is rejected because CTAP requires minPINLength to be at least four" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::UnexpectedType {
                            field: "minPINLength",
                            expected: "an unsigned integer greater than or equal to 4",
                        })
                    ));
                }
            }
            case "Rejects a maximum PIN length below the CTAP lower bound" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-maxpinlength"

                given "a GetInfo response advertising client PIN support with maxPINLength set to seven" {
                    let response = minimal().with_max_pin_length(7);
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "the response is rejected because a stated maxPINLength must be at least eight" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::UnexpectedType {
                            field: "maxPINLength",
                            expected: "an unsigned integer greater than or equal to 8",
                        })
                    ));
                }
            }
            case "Accepts a maximum PIN length above the independent 63-byte PIN limit" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-maxpinlength"

                given "a GetInfo response advertising maxPINLength as 64 Unicode code points" {
                    let response = minimal().with_max_pin_length(64);
                }
                when "the response is encoded and decoded through the CTAP wire representation" {
                    let encoded = response.to_cbor_bytes().unwrap();
                    let decoded = AuthenticatorGetInfoResponse::from_cbor_bytes(&encoded).unwrap();
                }
                then "the code-point limit is preserved independently of the PIN byte limit" {
                    assert_eq!(decoded.max_pin_length(), Some(64));
                }
            }
            case "Rejects a serialized large-blob capacity below 1024 bytes" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-maxserializedlargeblobarray"

                given "a GetInfo response whose maxSerializedLargeBlobArray is 1023 bytes" {
                    let response = minimal().with_max_serialized_large_blob_array(1023);
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "the response is rejected at the specification's 1024-byte lower bound" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::UnexpectedType {
                            field: "maxSerializedLargeBlobArray",
                            ..
                        })
                    ));
                }
            }
            case "Rejects a credential-blob capacity below 32 bytes" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-maxcredbloblength"

                given "a GetInfo response that advertises credBlob with a 31-byte maximum" {
                    let response = minimal()
                        .with_extensions(vec!["credBlob".to_owned()])
                        .with_max_credential_blob_length(31);
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "the response is rejected at the specification's 32-byte lower bound" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::UnexpectedType {
                            field: "maxCredBlobLength",
                            ..
                        })
                    ));
                }
            }
            case "Rejects a zero preferred platform UV attempt count" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-preferredplatformuvattempts"

                given "a GetInfo response whose preferredPlatformUvAttempts is zero" {
                    let response = minimal().with_preferred_platform_uv_attempts(0);
                }
                when "the response is encoded for the CTAP wire" {
                    let result = response.to_cbor_bytes();
                }
                then "the response is rejected because the attempt count must be positive" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::MustBePositive {
                            field: "preferredPlatformUvAttempts",
                        })
                    ));
                }
            }
            case "Rejects dependent GetInfo options without their prerequisite capabilities" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-nomcgapermissionswithclientpin"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-uvbioenroll"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-uvacfg"

                given "GetInfo responses that advertise a dependent option without its prerequisite option" {
                    let cases = [
                        ("noMcGaPermissionsWithClientPin", "clientPin"),
                        ("uvBioEnroll", "bioEnroll"),
                        ("uvAcfg", "authnrCfg"),
                    ];
                }
                when "each contradictory response is encoded" {
                    let results = cases.map(|(dependent, prerequisite)| {
                        let response = AuthenticatorGetInfoResponse::new(
                            vec!["FIDO_2_1".to_owned()],
                            [0; 16],
                        )
                        .with_options(BTreeMap::from([(dependent.to_owned(), false)]));
                        (dependent, prerequisite, response.to_cbor_bytes())
                    });
                }
                then "each response is rejected and identifies the missing prerequisite" {
                    for (dependent, prerequisite, result) in results {
                        assert!(
                            matches!(result, Err(ProtocolError::InvalidValue { field, .. }) if field == dependent),
                            "{dependent} should require {prerequisite}"
                        );
                    }
                }
            }
            case "Rejects mutually exclusive large-blob capability advertisements" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-largeblobs"

                given "a GetInfo response advertising both the largeBlob extension and largeBlobs command" {
                    let response = AuthenticatorGetInfoResponse::new(
                        vec!["FIDO_2_1".to_owned()],
                        [0; 16],
                    )
                    .with_extensions(vec!["largeBlob".to_owned()])
                    .with_options(BTreeMap::from([("largeBlobs".to_owned(), true)]))
                    .with_max_serialized_large_blob_array(1024);
                }
                when "the contradictory response is encoded" {
                    let result = response.to_cbor_bytes();
                }
                then "the response is rejected because the two large-blob mechanisms are exclusive" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::InvalidValue {
                            field: "options.largeBlobs/extensions.largeBlob",
                            ..
                        })
                    ));
                }
            }
            case "Requires makeCredUvNotRqd to be false when alwaysUv is enabled" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-alwaysuv"

                given "a GetInfo response enabling alwaysUv without explicitly disabling makeCredUvNotRqd" {
                    let response = AuthenticatorGetInfoResponse::new(
                        vec!["FIDO_2_1".to_owned()],
                        [0; 16],
                    )
                    .with_options(BTreeMap::from([("alwaysUv".to_owned(), true)]));
                }
                when "the contradictory response is encoded" {
                    let result = response.to_cbor_bytes();
                }
                then "the response is rejected because alwaysUv requires makeCredUvNotRqd=false" {
                    assert!(matches!(result, Err(ProtocolError::InvalidValue {
                            field: "options.makeCredUvNotRqd",
                            expected: "present and false when alwaysUv is true",
                        })));
                }
            }
            case "Rejects FIDO 2.3 capability sets that omit mandatory features" {
                spec "https://fidoalliance.org/specs/fido-v2.3-ps-20260226/fido-client-to-authenticator-protocol-v2.3-ps-20260226.html#mandatory-features"

                given "otherwise valid FIDO_2_3 GetInfo responses that independently omit protocol 2, hmac-secret, all verification methods, or pinUvAuthToken" {
                    let base = || {
                        AuthenticatorGetInfoResponse::new(
                            vec!["FIDO_2_3".to_owned()],
                            [0; 16],
                        )
                        .with_extensions(vec!["hmac-secret".to_owned()])
                        .with_options(BTreeMap::from([
                            ("clientPin".to_owned(), false),
                            ("pinUvAuthToken".to_owned(), true),
                            ("rk".to_owned(), true),
                            ("uv".to_owned(), false),
                        ]))
                        .with_min_pin_length(4)
                        .with_pin_uv_auth_protocols(vec![2])
                    };
                    let without_protocol_two = base().with_pin_uv_auth_protocols(vec![1]);
                    let without_hmac_secret = base().with_extensions(Vec::new());
                    let mut without_verification_options = base().with_options(BTreeMap::from([
                        ("pinUvAuthToken".to_owned(), true),
                        ("rk".to_owned(), true),
                    ]));
                    without_verification_options.min_pin_length = None;
                    let without_token_option = base().with_options(BTreeMap::from([
                        ("clientPin".to_owned(), true),
                        ("rk".to_owned(), true),
                        ("uv".to_owned(), false),
                    ]));
                }
                when "each capability set is encoded" {
                    let results = [
                        without_protocol_two.to_cbor_bytes(),
                        without_hmac_secret.to_cbor_bytes(),
                        without_verification_options.to_cbor_bytes(),
                        without_token_option.to_cbor_bytes(),
                    ];
                }
                then "each response is rejected at the mandatory FIDO 2.3 capability that it omitted" {
                    assert!(matches!(
                        &results[0],
                        Err(ProtocolError::InvalidValue {
                            field: "pinUvAuthProtocols",
                            ..
                        })
                    ));
                    assert!(matches!(
                        &results[1],
                        Err(ProtocolError::InvalidValue {
                            field: "extensions.hmac-secret",
                            ..
                        })
                    ));
                    assert!(matches!(
                        &results[2],
                        Err(ProtocolError::InvalidValue {
                            field: "options.clientPin/options.uv",
                            ..
                        })
                    ));
                    assert!(matches!(
                        &results[3],
                        Err(ProtocolError::InvalidValue {
                            field: "options.pinUvAuthToken",
                            ..
                        })
                    ));
                }
            }
            case "Rejects client PIN as a built-in UV modality" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-uvmodality"
                spec "https://fidoalliance.org/specs/common-specs/fido-registry-v2.2-ps-20220523.html#user-verification-methods"

                given "a GetInfo response advertising clientPin and the passcode_external registry bit in uvModality" {
                    let response = minimal().with_uv_modality(0x0000_0800);
                }
                when "the response is encoded" {
                    let result = response.to_cbor_bytes();
                }
                then "the response is rejected because client PIN is not a built-in verification method" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::InvalidValue {
                            field: "uvModality",
                            ..
                        })
                    ));
                }
            }
    }
}
