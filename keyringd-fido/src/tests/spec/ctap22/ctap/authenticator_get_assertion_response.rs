use crate::AuthenticatorData;
use crate::codec::CborCodec;
use crate::codec::CborItem as Value;
use crate::ctap::AuthenticationExtensionsAuthenticatorOutputs;
use crate::ctap::AuthenticatorGetAssertionResponse;
use crate::ctap::CtapValue;
use crate::ctap::ProtocolError;
use crate::ctap::PublicKeyCredentialDescriptor;
use crate::ctap::PublicKeyCredentialUserEntity;

keyringd_testkit::scenario! {
    group "AuthenticatorGetAssertionResponse::from_cbor_bytes" {
            case "Encodes every GetAssertion response member under its assigned integer key" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"

                given "a user-verified GetAssertion response containing every response member defined by CTAP" {
                    let response = AuthenticatorGetAssertionResponse {
                        credential: Some(PublicKeyCredentialDescriptor::new(
                            crate::webauthn::PublicKeyCredentialType::PublicKey,
                            vec![1, 2, 3],
                        )),
                        authenticator_data: AuthenticatorData::from_bytes(&{
                            let mut value = vec![0xaa; 32];
                            value.extend_from_slice(&[0x05, 0, 0, 0, 1]);
                            value
                        }).unwrap(),
                        signature: vec![0x30, 0x00],
                        user: Some(PublicKeyCredentialUserEntity::new(
                            "user@example.com",
                            vec![9],
                            "Example User",
                        )),
                        number_of_credentials: Some(1),
                        user_selected: Some(true),
                        large_blob_key: Some(vec![0x5a; 32]),
                        unsigned_extension_outputs: Some(
                            AuthenticationExtensionsAuthenticatorOutputs::default().with_value(
                                "example",
                                crate::EncodedCborValue::from_canonical_bytes(vec![0xf5]).unwrap(),
                            ),
                        ),
                    };
                }
                when "the response is encoded as a canonical CTAP CBOR map" {
                    let encoded = response.to_cbor_bytes().unwrap();
                    let fields = CtapValue::ctap_map(CborCodec::decode(&encoded).unwrap())
                        .unwrap();
                }
                then "every key contains the value and CBOR type assigned by authenticatorGetAssertion" {
                    let canonical = |value| {
                        CborCodec::decode(&CborCodec::encode(value).unwrap()).unwrap()
                    };
                    assert_eq!(fields.keys().copied().collect::<Vec<_>>(), (1..=8).collect::<Vec<_>>());
                    assert_eq!(
                        fields[&1],
                        canonical(response.credential.as_ref().unwrap().to_cbor_value().unwrap())
                    );
                    assert_eq!(
                        fields[&2],
                        Value::ByteString(response.authenticator_data.to_bytes().unwrap())
                    );
                    assert_eq!(fields[&3], Value::ByteString(vec![0x30, 0x00]));
                    assert_eq!(
                        fields[&4],
                        canonical(response.user.as_ref().unwrap().to_cbor_value().unwrap())
                    );
                    assert_eq!(fields[&5], Value::Unsigned(1));
                    assert_eq!(fields[&6], Value::bool_value(true));
                    assert_eq!(fields[&7], Value::ByteString(vec![0x5a; 32]));
                    assert_eq!(
                        fields[&8],
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
                        AuthenticatorGetAssertionResponse::from_cbor_bytes(&encoded).unwrap(),
                        response
                    );
                }
            }
            case "Treats an empty unsigned extension output map as omitted" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"

                given "a GetAssertion response that explicitly contains an empty unsigned extension output map" {
                    let encoded = hex_literal::hex!(
                        "a4
                         01 a2 62 6964 41 01 64 74797065 6a 7075626c69632d6b6579
                         02 58 25
                            0000000000000000000000000000000000000000000000000000000000000000
                            01
                            00000000
                         03 41 00
                         08 a0"
                    );
                }
                when "the response is decoded" {
                    let response = AuthenticatorGetAssertionResponse::from_cbor_bytes(&encoded)
                        .unwrap();
                }
                then "the client exposes the empty map exactly like an omitted field" {
                    assert!(response.unsigned_extension_outputs().is_none());
                }
            }
            case "Assertion response can be converted back and forth" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"

                given "Prepare Credential, Authenticator Data, signed Assertion responses" {
                    let response = AuthenticatorGetAssertionResponse {
                        credential: Some(PublicKeyCredentialDescriptor::new(
                            crate::webauthn::PublicKeyCredentialType::PublicKey,
                            vec![1, 2, 3],
                        )),
                        authenticator_data: AuthenticatorData::from_bytes(&{
                            let mut value = vec![0xaa; 32];
                            value.extend_from_slice(&[0x05, 0, 0, 0, 1]);
                            value
                        })
                        .unwrap(),
                        signature: vec![0x30, 0x00],
                        user: Some(PublicKeyCredentialUserEntity::from_id(vec![9])),
                        number_of_credentials: Some(2),
                        user_selected: None,
                        large_blob_key: None,
                        unsigned_extension_outputs: None,
                    };
                }
                when "Encode the response to CBOR and then decode" {
                    let decoded = AuthenticatorGetAssertionResponse::from_cbor_bytes(
                        &response.to_cbor_bytes().unwrap(),
                    )
                    .unwrap();
                }
                then "The original Assertion response is restored" {
                    assert_eq!(
                        decoded,
                        response
                    );
                    assert_eq!(decoded.user_selected(), None);
                }

            }
            case "Retains the omitted credential permitted by CTAP 2.0" {
                spec "https://fidoalliance.org/specs/fido-v2.0-ps-20190130/fido-client-to-authenticator-protocol-v2.0-ps-20190130.html#authenticatorGetAssertion"
                verify "wire parsing and encoding leave credential resolution to the client" {
                    let response = AuthenticatorGetAssertionResponse::new(None, AuthenticatorData::new("example.com", 1, 0).unwrap(), vec![1]);
                    let bytes = response.to_cbor_bytes().unwrap();
                    let decoded = AuthenticatorGetAssertionResponse::from_cbor_bytes(&bytes).unwrap();
                    assert!(decoded.credential().is_none());
                    assert_eq!(decoded, response);
                    assert_eq!(decoded.to_cbor_bytes().unwrap(), bytes);
                }
            }
            case "Rejects an assertion response without authenticator data" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"

                verify "AuthData rejects missing responses" {
                    assert!(matches!(AuthenticatorGetAssertionResponse::from_cbor_bytes(&[0xa0]), Err(ProtocolError::MissingCtapField(2))));
                    }

            }
            case "Rejects user-identifying fields when user verification was not performed" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"

                given "an assertion with the UP flag only and a user entity containing names" {
                    let response = AuthenticatorGetAssertionResponse::new(Some(PublicKeyCredentialDescriptor::new(
                            crate::webauthn::PublicKeyCredentialType::PublicKey,
                            vec![1],
                        )),
                        AuthenticatorData::from_bytes(&{
                            let mut value = vec![0xaa; 32];
                            value.extend_from_slice(&[0x01, 0, 0, 0, 1]);
                            value
                        })
                        .unwrap(),
                        vec![0x30, 0x00],
                    )
                    .with_user(PublicKeyCredentialUserEntity::new(
                        "private-name",
                        vec![9],
                        "Private Display Name",
                    ));
                }
                when "the assertion response is encoded" {
                    let result = response.to_cbor_bytes();
                }
                then "the response is rejected instead of exposing identifying account data without UV" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::UnexpectedType { field: "user", .. })
                    ));
                }
            }
            case "Ignores user-identifying fields received without user verification" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"

                given "an encoded assertion with the UP flag only and a user entity containing names" {
                    let credential = PublicKeyCredentialDescriptor::new(
                        crate::webauthn::PublicKeyCredentialType::PublicKey,
                        vec![1],
                    );
                    let mut auth_data = vec![0xaa; 32];
                    auth_data.extend_from_slice(&[0x01, 0, 0, 0, 1]);
                    let user = PublicKeyCredentialUserEntity::new(
                        "private-name",
                        vec![9],
                        "Private Display Name",
                    );
                    let encoded = CborCodec::encode(Value::Map(vec![
                        (1u64.into(), credential.to_cbor_value().unwrap()),
                        (2u64.into(), Value::ByteString(auth_data)),
                        (3u64.into(), Value::ByteString(vec![0x30, 0x00])),
                        (4u64.into(), user.to_cbor_value().unwrap()),
                    ])).unwrap();
                }
                when "the assertion response is decoded by a client" {
                    let response = AuthenticatorGetAssertionResponse::from_cbor_bytes(&encoded)
                        .unwrap();
                }
                then "the identifying fields are discarded while the user ID is retained" {
                    let user = response.user().unwrap();
                    assert_eq!(user.id(), &[9]);
                    assert_eq!(user.name(), None);
                    assert_eq!(user.display_name(), None);
                }
            }
            case "Rejects userSelected when multiple credentials remain" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"

                given "an assertion that reports multiple credentials and direct user selection together" {
                    let response = AuthenticatorGetAssertionResponse::new(Some(PublicKeyCredentialDescriptor::new(
                            crate::webauthn::PublicKeyCredentialType::PublicKey,
                            vec![1],
                        )),
                        AuthenticatorData::from_bytes(&{
                            let mut value = vec![0xaa; 32];
                            value.extend_from_slice(&[0x01, 0, 0, 0, 1]);
                            value
                        })
                        .unwrap(),
                        vec![0x30, 0x00],
                    )
                    .with_number_of_credentials(2)
                    .with_user_selected(true);
                }
                when "the contradictory assertion response is encoded" {
                    let result = response.to_cbor_bytes();
                }
                then "the mutually exclusive response fields are rejected" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::UnexpectedType { field: "userSelected", .. })
                    ));
                }
            }
            case "Rejects an assertion large-blob key with the wrong length" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"

                given "an assertion response containing a 33-byte largeBlobKey" {
                    let response = AuthenticatorGetAssertionResponse::new(Some(PublicKeyCredentialDescriptor::new(
                            crate::webauthn::PublicKeyCredentialType::PublicKey,
                            vec![1],
                        )),
                        AuthenticatorData::from_bytes(&{
                            let mut value = vec![0xaa; 32];
                            value.extend_from_slice(&[0x01, 0, 0, 0, 1]);
                            value
                        })
                        .unwrap(),
                        vec![0x30, 0x00],
                    )
                    .with_large_blob_key(vec![0; 33]);
                }
                when "the response is encoded" {
                    let result = response.to_cbor_bytes();
                }
                then "the key is rejected because CTAP defines largeBlobKey as a 32-byte key" {
                    assert!(matches!(result, Err(ProtocolError::InvalidLength {
                            field: "largeBlobKey",
                            expected: 32,
                            actual: 33,
                        })));
                }
            }
    }
}
