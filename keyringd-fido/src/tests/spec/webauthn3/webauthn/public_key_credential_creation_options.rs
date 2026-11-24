use crate::webauthn::AttestationConveyancePreference;
use crate::webauthn::ProtocolError;
use crate::webauthn::PublicKeyCredentialCreationOptions;
use crate::webauthn::PublicKeyCredentialParameters;
use crate::webauthn::PublicKeyCredentialRpEntity;
use crate::webauthn::PublicKeyCredentialType;
use crate::webauthn::PublicKeyCredentialUserEntity;

keyringd_testkit::scenario! {
    case "Creation options validate the required WebAuthn user names" {
        given "CTAP users with one or both WebAuthn names missing" {
            let users = [
                (PublicKeyCredentialUserEntity::from_id(vec![1]), "user.name"),
                (
                    PublicKeyCredentialUserEntity::from_id(vec![1]).with_name("user"),
                    "user.displayName",
                ),
            ];
        }
        when "the caller validates creation options" {
            let results = users.map(|(user, field)| {
                let options = PublicKeyCredentialCreationOptions::new(
                    PublicKeyCredentialRpEntity::new("RP").with_id("example.com"),
                    user,
                    vec![2],
                    vec![PublicKeyCredentialParameters::es256()],
                );
                (field, options.validate(), options.to_json_bytes())
            });
        }
        then "validation and JSON encoding both reject the missing name" {
            for (field, validation, encoding) in results {
                assert!(matches!(validation, Err(ProtocolError::MissingField(actual_field)) if actual_field == field));
                assert!(matches!(encoding, Err(ProtocolError::MissingField(actual_field)) if actual_field == field));
            }
        }
    }
        case "Constructor Apply Web IDL Defaults" {
            spec "https://www.w3.org/TR/webauthn-3/#dictionary-makecredentialoptions"

            given "Prepare Credential Creation Options for Required Arguments" {
                let relying_party = PublicKeyCredentialRpEntity::new("RP").with_id("example.com");
                let user = PublicKeyCredentialUserEntity::new("user", vec![1], "User");
                let parameters = vec![PublicKeyCredentialParameters::new(
                    PublicKeyCredentialType::PublicKey,
                    crate::webauthn::CoseAlgorithmIdentifier::ES256,
                )];
            }
            when "Generate Options in Constructor" {
                let options = PublicKeyCredentialCreationOptions::new(
                    relying_party,
                    user,
                    vec![2],
                    parameters,
                );
            }
            then "Web IDL defaults apply to any field" {
                assert_eq!(options.timeout(), None);
                assert!(options.exclude_credentials().is_empty());
                assert!(options.hints().is_empty());
                assert_eq!(options.attestation(), &AttestationConveyancePreference::None);
                assert!(options.attestation_formats().is_empty());
                assert_eq!(options.extensions(), None);
            }

        }
}

keyringd_testkit::scenario! {
        case "JSON can be converted back and forth as a semantic type" {
            spec "https://www.w3.org/TR/webauthn-3/#dictionary-makecredentialoptions"
            spec "https://www.w3.org/TR/webauthn-3/#sctn-parseCreationOptionsFromJSON"

            given "Prepare a JSON for Credential Creation Options including Binary String" {
                let json = br#"{"rp":{"name":"Example"},"user":{"id":"AQ","name":"u","displayName":"U"},"challenge":"Ag","pubKeyCredParams":[]}"#;
            }
            when "Decode JSON to semantic type, encode again and decode" {
                let options = PublicKeyCredentialCreationOptions::from_json_bytes(
                    json,
                )
                .unwrap();
                let encoded = options.to_json_bytes().unwrap();
                let decoded = PublicKeyCredentialCreationOptions::from_json_bytes(&encoded).unwrap();
            }
            then "Binary String and Options as a whole" {
                assert_eq!(options.user().id(), [1]);
                assert_eq!(options.challenge(), [2]);
                assert_eq!(
                    decoded,
                    options
                );
            }

        }
}

keyringd_testkit::scenario! {
    case "Rejects incompatible registration options JSON" {
        spec "https://www.w3.org/TR/webauthn-3/#sctn-parseCreationOptionsFromJSON"

        verify "a padded base64 user identifier is rejected instead of being partially converted" {
            let json = br#"{"rp":{"name":"Example"},"user":{"id":"AQ==","name":"u","displayName":"U"},"challenge":"Ag","pubKeyCredParams":[]}"#;
            assert!(PublicKeyCredentialCreationOptions::from_json_bytes(json).is_err());
        }
    }

    case "Ignores unsupported registration extensions while retaining supported inputs" {
        spec "https://www.w3.org/TR/webauthn-3/#sctn-createCredential"

        given "registration JSON containing credProps and an unsupported extension" {
            let json = br#"{"rp":{"name":"Example"},"user":{"id":"AQ","name":"u","displayName":"U"},"challenge":"Ag","pubKeyCredParams":[],"extensions":{"credProps":true,"futureExtension":{"x":1}}}"#;
        }
        when "the client model converts the registration options" {
            let options = PublicKeyCredentialCreationOptions::from_json_bytes(json).unwrap();
            let encoded: serde_json::Value =
                serde_json::from_slice(&options.to_json_bytes().unwrap()).unwrap();
        }
        then "the supported input is retained and the unsupported extension is absent" {
            assert_eq!(
                options.extensions().and_then(|value| value.credential_properties()),
                Some(true)
            );
            assert!(encoded["extensions"].get("futureExtension").is_none());
        }
    }

}
