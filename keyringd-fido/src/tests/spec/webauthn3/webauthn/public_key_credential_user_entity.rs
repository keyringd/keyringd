use crate::ProtocolError;
use crate::codec::CborItem as Value;
use crate::webauthn::PublicKeyCredentialUserEntity;

keyringd_testkit::scenario! {
    group "PublicKeyCredentialUserEntity context-specific requirements" {
        case "WebAuthn and CTAP enforce their respective user ID boundaries" {
            given "user IDs at and outside the permitted boundaries" {
                let lengths = [0, 1, 64, 65];
            }
            when "each ID is validated and encoded for both representations" {
                let results = lengths.map(|length| {
                    let ctap = PublicKeyCredentialUserEntity::from_id(vec![0; length]);
                    let cbor = Value::Map(vec![(
                        Value::TextString("id".to_owned()),
                        Value::ByteString(vec![0; length]),
                    )]);
                    let webauthn = PublicKeyCredentialUserEntity::new(
                        "user", vec![0; length], "User",
                    );
                    (
                        length,
                        ctap.to_cbor_value().is_ok(),
                        PublicKeyCredentialUserEntity::from_cbor_value(cbor).is_ok(),
                        webauthn.to_json_value().is_ok(),
                    )
                });
            }
            then "CTAP accepts empty IDs while both reject IDs longer than 64 bytes" {
                for (length, ctap_encode, ctap_decode, webauthn_encode) in results {
                    assert_eq!(ctap_encode, length <= 64);
                    assert_eq!(ctap_decode, length <= 64);
                    assert_eq!(webauthn_encode, (1..=64).contains(&length));
                }
                assert!(matches!(
                    PublicKeyCredentialUserEntity::from_json_value(serde_json::json!({
                        "id": "", "name": "user", "displayName": "User",
                    })),
                    Err(ProtocolError::EmptyValue { field: "user.id" }),
                ));
            }
        }
            case "Requires names in WebAuthn while permitting an ID-only CTAP entity" {
                spec "https://www.w3.org/TR/webauthn-3/#dictionary-user-credential-params"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorMakeCredential"

                given "a complete WebAuthn user entity and a privacy-preserving CTAP user entity" {
                    let webauthn = PublicKeyCredentialUserEntity::new(
                        "renee@example.com",
                        vec![1, 2, 3],
                        "Renée",
                    );
                    let ctap = PublicKeyCredentialUserEntity::from_id(vec![1, 2, 3]);
                }
                when "each entity is encoded in its corresponding representation" {
                    let json = webauthn.to_json_value().unwrap();
                    let cbor = ctap.to_cbor_value().unwrap();
                    let decoded = PublicKeyCredentialUserEntity::from_cbor_value(cbor.clone()).unwrap();
                }
                then "WebAuthn includes both names and CTAP can contain only the byte-string user ID" {
                    assert_eq!(
                        json,
                        serde_json::json!({
                            "name": "renee@example.com",
                            "id": "AQID",
                            "displayName": "Renée"
                        })
                    );
                    assert_eq!(
                        cbor,
                        Value::Map(vec![(
                            Value::TextString("id".to_owned()),
                            Value::ByteString(vec![1, 2, 3]),
                        )])
                    );
                    assert_eq!(decoded, ctap);
                }
            }
    }
}
