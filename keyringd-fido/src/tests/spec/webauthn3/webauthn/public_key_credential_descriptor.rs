use crate::codec::CborItem as Value;
use crate::webauthn::AuthenticatorTransport;
use crate::webauthn::PublicKeyCredentialDescriptor;
use crate::webauthn::PublicKeyCredentialType;

keyringd_testkit::scenario! {
    group "PublicKeyCredentialDescriptor wire representations" {
            case "Uses base64url in WebAuthn JSON and bytes in CTAP CBOR" {
                spec "https://www.w3.org/TR/webauthn-3/#dictdef-publickeycredentialdescriptor"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#makecred-excludelist"

                given "a public-key credential descriptor with USB and hybrid transports" {
                    let descriptor = PublicKeyCredentialDescriptor::new(
                        PublicKeyCredentialType::PublicKey,
                        vec![1, 2, 3],
                    )
                    .with_transports(vec![
                        AuthenticatorTransport::usb(),
                        AuthenticatorTransport::hybrid(),
                    ]);
                }
                when "the descriptor is encoded for WebAuthn JSON and CTAP CBOR" {
                    let json = descriptor.to_json_value().unwrap();
                    let cbor = descriptor.to_cbor_value().unwrap();
                    let json_round_trip = PublicKeyCredentialDescriptor::from_json_value(json.clone()).unwrap();
                    let cbor_round_trip = PublicKeyCredentialDescriptor::from_cbor_value(cbor.clone()).unwrap();
                }
                then "JSON uses a base64url credential ID while CBOR keeps the credential ID as a byte string" {
                    assert_eq!(
                        json,
                        serde_json::json!({
                            "type": "public-key",
                            "id": "AQID",
                            "transports": ["usb", "hybrid"]
                        })
                    );
                    let Value::Map(fields) = cbor else { panic!("descriptor must be a CBOR map") };
                    assert!(fields.contains(&(
                        Value::TextString("id".to_owned()),
                        Value::ByteString(vec![1, 2, 3]),
                    )));
                    assert_eq!(json_round_trip, descriptor);
                    assert_eq!(cbor_round_trip, descriptor);
                }
            }
    }
}
