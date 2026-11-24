use crate::webauthn::*;

keyringd_testkit::scenario! {
    group "AuthenticatorAssertionResponse JSON representation" {
            case "Round-trips every assertion response member as base64url text" {
                spec "https://www.w3.org/TR/webauthn-3/#dictdef-authenticatorassertionresponsejson"

                given "an assertion response containing client data, authenticator data, a signature, and a user handle" {
                    let response = AuthenticatorAssertionResponse::new(
                        vec![1, 2, 3],
                        vec![4, 5],
                        vec![6],
                        Some(vec![7, 8]),
                    );
                }
                when "the response is encoded to the WebAuthn JSON representation" {
                    let bytes = response.to_json_bytes().unwrap();
                    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                    let decoded = AuthenticatorAssertionResponse::from_json_bytes(&bytes).unwrap();
                }
                then "all binary members use unpadded base64url and decode without loss" {
                    assert_eq!(
                        json,
                        serde_json::json!({
                            "clientDataJSON": "AQID",
                            "authenticatorData": "BAU",
                            "signature": "Bg",
                            "userHandle": "Bwg"
                        })
                    );
                    assert_eq!(decoded, response);
                }
            }
    }
}
