use crate::CodecError;
use crate::ProtocolError;
use crate::webauthn::PublicKeyCredentialRequestOptions;
use crate::webauthn::UserVerificationRequirement;

keyringd_testkit::scenario! {
        case "Constructor Apply Web IDL Defaults" {
            spec "https://www.w3.org/TR/webauthn-3/#dictionary-assertion-options"

            given "Prepare Credential Request Options that specify only CHALLENGE" {
                let challenge = vec![1, 2, 3];
            }
            when "Generate Options in Constructor" {
                let options = PublicKeyCredentialRequestOptions::new(challenge);
            }
            then "Web IDL defaults apply to any field" {
                assert_eq!(options.timeout(), None);
                assert_eq!(options.relying_party_id(), None);
                assert!(options.allow_credentials().is_empty());
                assert_eq!(
                    options.user_verification(),
                    &UserVerificationRequirement::Preferred
                );
                assert!(options.hints().is_empty());
                assert_eq!(options.extensions(), None);
            }

        }
}

keyringd_testkit::scenario! {
        case "Rejects non-normal format Binary String" {
            spec "https://www.w3.org/TR/webauthn-3/#dictionary-assertion-options"
            spec "https://www.w3.org/TR/webauthn-3/#sctn-parseRequestOptionsFromJSON"

            verify "Deny non-normal format Binary String" {
                assert!(matches!(
                    PublicKeyCredentialRequestOptions::from_json_bytes(br#"{"challenge":"AQ=="}"#),
                    Err(ProtocolError::Codec(
                        CodecError::InvalidBase64Url(_) | CodecError::NonCanonicalBase64Url
                    ))
                ));
                }

        }
}

keyringd_testkit::scenario! {
    case "Decodes authentication options JSON and ignores unsupported extensions" {
        spec "https://www.w3.org/TR/webauthn-3/#sctn-parseRequestOptionsFromJSON"
        spec "https://www.w3.org/TR/webauthn-3/#sctn-discover-from-external-source"

        given "authentication JSON with base64url binary fields and an unsupported extension" {
            let json = br#"{"challenge":"AQI","allowCredentials":[{"type":"public-key","id":"AwQ"}],"extensions":{"futureExtension":true}}"#;
        }
        when "the client model converts and re-encodes the authentication options" {
            let options = PublicKeyCredentialRequestOptions::from_json_bytes(json).unwrap();
            let encoded: serde_json::Value =
                serde_json::from_slice(&options.to_json_bytes().unwrap()).unwrap();
        }
        then "binary strings are decoded and no value is emitted for the ignored extension" {
            assert_eq!(options.challenge(), [1, 2]);
            assert_eq!(options.allow_credentials()[0].id(), [3, 4]);
            assert_eq!(encoded["extensions"], serde_json::json!({}));
        }
    }
}
