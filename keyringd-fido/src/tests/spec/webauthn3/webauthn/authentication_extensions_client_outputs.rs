use crate::webauthn::AuthenticationExtensionsClientOutputs;
use crate::webauthn::CredentialPropertiesOutput;

keyringd_testkit::scenario! {
    group "AuthenticationExtensionsClientOutputs JSON representation" {
            case "Preserves an explicit true discoverable credential result" {
                spec "https://www.w3.org/TR/webauthn-3/#dictdef-credentialpropertiesoutput"

                given "a credProps result whose rk member reports true" {
                    let json = serde_json::json!({"credProps": {"rk": true}});
                }
                when "the client extension output is decoded and encoded again" {
                    let outputs = AuthenticationExtensionsClientOutputs::from_json_value(json.clone()).unwrap();
                    let encoded = outputs.to_json_value().unwrap();
                }
                then "true remains the discoverable-credential result" {
                    assert_eq!(
                        outputs
                            .credential_properties()
                            .and_then(CredentialPropertiesOutput::rk),
                        Some(true)
                    );
                    assert_eq!(encoded, json);
                }
            }
            case "Preserves an explicit false discoverable credential result" {
                spec "https://www.w3.org/TR/webauthn-3/#sctn-authenticator-credential-properties-extension"

                given "a credProps result whose rk member explicitly reports false" {
                    let json = serde_json::json!({"credProps": {"rk": false}});
                }
                when "the client extension output is decoded and encoded again" {
                    let outputs = AuthenticationExtensionsClientOutputs::from_json_value(json.clone()).unwrap();
                    let encoded = outputs.to_json_value().unwrap();
                }
                then "false remains distinct from an omitted rk member" {
                    assert_eq!(
                        outputs
                            .credential_properties()
                            .and_then(CredentialPropertiesOutput::rk),
                        Some(false)
                    );
                    assert_eq!(encoded, json);
                }
            }
            case "Preserves an omitted discoverable credential result" {
                spec "https://www.w3.org/TR/webauthn-3/#dictdef-credentialpropertiesoutput"

                given "a credProps result that omits the optional rk member" {
                    let json = serde_json::json!({"credProps": {}});
                }
                when "the client extension output is decoded and encoded again" {
                    let outputs = AuthenticationExtensionsClientOutputs::from_json_value(json.clone()).unwrap();
                    let encoded = outputs.to_json_value().unwrap();
                }
                then "the absence remains distinguishable from rk false" {
                    assert_eq!(
                        outputs
                            .credential_properties()
                            .and_then(CredentialPropertiesOutput::rk),
                        None
                    );
                    assert_eq!(encoded, json);
                }
            }
    }
}
