use crate::codec::CborItem as Value;
use crate::webauthn::PublicKeyCredentialParameters;

keyringd_testkit::scenario! {
    group "PublicKeyCredentialParameters wire representations" {
            case "Encodes ES256 with its registered COSE algorithm identifier" {
                spec "https://www.w3.org/TR/webauthn-3/#dictdef-publickeycredentialparameters"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#makecred-pubkeycredparams"

                given "the ES256 public-key credential parameters" {
                    let parameters = PublicKeyCredentialParameters::es256();
                }
                when "the parameters are encoded as JSON and CTAP CBOR values" {
                    let json = serde_json::to_value(&parameters).unwrap();
                    let cbor = parameters.to_cbor_value().unwrap();
                }
                then "both representations contain type public-key and algorithm negative seven" {
                    assert_eq!(json, serde_json::json!({ "type": "public-key", "alg": -7 }));
                    let Value::Map(fields) = cbor else { panic!("parameters must be a CBOR map") };
                    assert!(fields.contains(&(
                        Value::TextString("type".to_owned()),
                        Value::TextString("public-key".to_owned()),
                    )));
                    assert!(fields.contains(&(
                        Value::TextString("alg".to_owned()),
                        Value::Negative(-7),
                    )));
                }
            }
    }
}
