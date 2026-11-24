use crate::codec::CborItem as Value;
use crate::webauthn::PublicKeyCredentialRpEntity;

keyringd_testkit::scenario! {
    group "PublicKeyCredentialRpEntity context-specific requirements" {
            case "Requires a name in WebAuthn while permitting an ID-only CTAP entity" {
                spec "https://www.w3.org/TR/webauthn-3/#dictionary-rp-credential-params"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorMakeCredential"

                given "a WebAuthn RP entity and a privacy-preserving CTAP RP entity" {
                    let webauthn = PublicKeyCredentialRpEntity::new("Example")
                        .with_id("example.com");
                    let ctap = PublicKeyCredentialRpEntity::from_rp_id("example.com");
                }
                when "each entity is encoded in its corresponding representation" {
                    let json = serde_json::to_value(&webauthn).unwrap();
                    let cbor = ctap.to_cbor_value().unwrap();
                    let decoded = PublicKeyCredentialRpEntity::from_cbor_value(cbor.clone()).unwrap();
                }
                then "WebAuthn includes the human-readable name and CTAP can contain only the RP ID" {
                    assert_eq!(json, serde_json::json!({ "name": "Example", "id": "example.com" }));
                    assert_eq!(
                        cbor,
                        Value::Map(vec![(
                            Value::TextString("id".to_owned()),
                            Value::TextString("example.com".to_owned()),
                        )])
                    );
                    assert_eq!(decoded, ctap);
                }
            }
    }
}
