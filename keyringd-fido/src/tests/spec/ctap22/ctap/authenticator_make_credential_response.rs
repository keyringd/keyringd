use crate::AttestationStatement;
use crate::AuthenticatorData;
use crate::EncodedCborValue;
use crate::codec::CborCodec;
use crate::codec::CborItem as Value;
use crate::ctap::AuthenticationExtensionsAuthenticatorOutputs;
use crate::ctap::AuthenticatorMakeCredentialResponse;
use crate::ctap::CtapValue;
use crate::ctap::ProtocolError;

keyringd_testkit::scenario! {
    group "AuthenticatorMakeCredentialResponse::from_cbor_bytes" {
            case "Encodes every MakeCredential response member under its assigned integer key" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorMakeCredential"

                given "a MakeCredential response containing every response member defined by CTAP" {
                    let response = AuthenticatorMakeCredentialResponse {
                        format: "none".to_owned(),
                        authenticator_data: AuthenticatorData::from_bytes(&{
                            let mut value = vec![0xbb; 32];
                            value.extend_from_slice(&[0x01, 0, 0, 0, 0]);
                            value
                        }).unwrap(),
                        attestation_statement: Some(AttestationStatement::None),
                        enterprise_attestation: Some(true),
                        large_blob_key: Some(vec![0x5a; 32]),
                        unsigned_extension_outputs: Some(
                            AuthenticationExtensionsAuthenticatorOutputs::default().with_value(
                                "example",
                                EncodedCborValue::from_canonical_bytes(vec![0xf5]).unwrap(),
                            ),
                        ),
                    };
                }
                when "the response is encoded as a canonical CTAP CBOR map" {
                    let encoded = response.to_cbor_bytes().unwrap();
                    let fields = CtapValue::ctap_map(CborCodec::decode(&encoded).unwrap())
                        .unwrap();
                }
                then "every key contains the value and CBOR type assigned by authenticatorMakeCredential" {
                    assert_eq!(fields.keys().copied().collect::<Vec<_>>(), (1..=6).collect::<Vec<_>>());
                    assert_eq!(fields[&1], Value::TextString("none".to_owned()));
                    assert_eq!(
                        fields[&2],
                        Value::ByteString(response.authenticator_data.to_bytes().unwrap())
                    );
                    assert_eq!(
                        fields[&3],
                        CborCodec::decode(
                            &response
                                .attestation_statement
                                .as_ref()
                                .unwrap()
                                .to_cbor_bytes()
                                .unwrap(),
                        )
                        .unwrap()
                    );
                    assert_eq!(fields[&4], Value::bool_value(true));
                    assert_eq!(fields[&5], Value::ByteString(vec![0x5a; 32]));
                    assert_eq!(
                        fields[&6],
                        CborCodec::decode(
                            &response
                                .unsigned_extension_outputs
                                .as_ref()
                                .unwrap()
                                .to_cbor_bytes()
                                .unwrap(),
                        )
                        .unwrap()
                    );
                    assert_eq!(
                        AuthenticatorMakeCredentialResponse::from_cbor_bytes(&encoded).unwrap(),
                        response
                    );
                }
            }
            case "Treats an empty unsigned extension output map as omitted" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorMakeCredential"

                given "a MakeCredential response that explicitly contains an empty unsigned extension output map" {
                    let encoded = hex_literal::hex!(
                        "a3
                         01 64 6e6f6e65
                         02 58 25
                            0000000000000000000000000000000000000000000000000000000000000000
                            01
                            00000000
                         06 a0"
                    );
                }
                when "the response is decoded" {
                    let response = AuthenticatorMakeCredentialResponse::from_cbor_bytes(&encoded)
                        .unwrap();
                }
                then "the client exposes the empty map exactly like an omitted field" {
                    assert!(response.unsigned_extension_outputs().is_none());
                }
            }
            case "None Attestation Response can be converted back and forth" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorMakeCredential"
                spec "https://www.w3.org/TR/webauthn-3/#sctn-none-attestation"

                given "Prepare a MakeCredential Response with None Attestation" {
                    let response = AuthenticatorMakeCredentialResponse {
                        format: "none".to_owned(),
                        authenticator_data: AuthenticatorData::from_bytes(&{
                            let mut value = vec![0xbb; 32];
                            value.extend_from_slice(&[0x01, 0, 0, 0, 0]);
                            value
                        })
                        .unwrap(),
                        attestation_statement: Some(AttestationStatement::None),
                        enterprise_attestation: None,
                        large_blob_key: None,
                        unsigned_extension_outputs: None,
                    };
                }
                when "Encode the response to CBOR and then decode" {
                    let decoded = AuthenticatorMakeCredentialResponse::from_cbor_bytes(
                        &response.to_cbor_bytes().unwrap(),
                    )
                    .unwrap();
                }
                then "The original MakeCredential response is restored" {
                    assert_eq!(
                        decoded,
                        response
                    );
                }

            }
            case "Accepts an attestation response without an attestation statement" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorMakeCredential"

                given "Prepare a CBOR response without an Attestation Statement field" {
                    let encoded = hex_literal::hex!(
                        "a2
                         01 64 6e6f6e65
                         02 58 25
                            0000000000000000000000000000000000000000000000000000000000000000
                            01
                            00000000"
                    );
                }
                when "Decode the response" {
                    let response = AuthenticatorMakeCredentialResponse::from_cbor_bytes(&encoded).unwrap();
                }
                then "Accepted as a response without Attestation Statement" {
                    assert!(response.attestation_statement().is_none());
                }

            }
            case "Rejects a registration large-blob key with the wrong length" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorMakeCredential"

                given "a MakeCredential response containing a 31-byte largeBlobKey" {
                    let response = AuthenticatorMakeCredentialResponse {
                        format: "none".to_owned(),
                        authenticator_data: AuthenticatorData::from_bytes(&{
                            let mut value = vec![0xbb; 32];
                            value.extend_from_slice(&[0x01, 0, 0, 0, 0]);
                            value
                        })
                        .unwrap(),
                        attestation_statement: Some(AttestationStatement::None),
                        enterprise_attestation: None,
                        large_blob_key: Some(vec![0; 31]),
                        unsigned_extension_outputs: None,
                    };
                }
                when "the response is encoded" {
                    let result = response.to_cbor_bytes();
                }
                then "the key is rejected because CTAP defines largeBlobKey as a 32-byte key" {
                    assert!(matches!(result, Err(ProtocolError::InvalidLength {
                            field: "largeBlobKey",
                            expected: 32,
                            actual: 31,
                        })));
                }
            }
    }
}
