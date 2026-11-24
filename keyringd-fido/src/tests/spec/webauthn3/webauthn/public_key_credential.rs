use crate::webauthn::*;

keyringd_testkit::scenario! {
        case "Authentication Response can be transformed as a semantic type" {
            spec "https://www.w3.org/TR/webauthn-3/#iface-pkcredential"
            spec "https://www.w3.org/TR/webauthn-3/#iface-authenticatorassertionresponse"

            given "Prepare a PublicKeyCredential JSON that contains an Authentication response" {
                let json = br#"{"id":"AQ","rawId":"AQ","response":{"clientDataJSON":"Ag","authenticatorData":"Aw","signature":"BA"},"clientExtensionResults":{},"type":"public-key"}"#;
            }
            when "Decode JSON to semantic type and decode again via JSON" {
                let credential = PublicKeyCredential::<AuthenticatorAssertionResponse>::from_json_bytes(
                    json,
                )
                .unwrap();
                let encoded = credential.to_json_bytes().unwrap();
                let decoded = PublicKeyCredential::<AuthenticatorAssertionResponse>::from_json_bytes(&encoded)
                    .unwrap();
            }
            then "BINARY STRING and CREDENTIAL as a whole are retained" {
                let json_value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
                assert_eq!(credential.id(), "AQ");
                assert_eq!(credential.raw_id(), [1]);
                assert_eq!(credential.response().signature(), [4]);
                assert_eq!(json_value["rawId"], "AQ");
                assert_eq!(json_value["response"]["clientDataJSON"], "Ag");
                assert_eq!(json_value["response"]["authenticatorData"], "Aw");
                assert_eq!(json_value["response"]["signature"], "BA");
                assert_eq!(
                    decoded,
                    credential
                );
            }

        }
}
