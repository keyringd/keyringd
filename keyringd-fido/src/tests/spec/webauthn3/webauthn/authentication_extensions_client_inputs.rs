use crate::webauthn::AuthenticationExtensionsClientInputs;
use crate::webauthn::CredentialProtectionPolicy;

keyringd_testkit::scenario! {
        case "Represents the credential properties and credential protection client inputs" {
            spec "https://www.w3.org/TR/webauthn-3/#sctn-authenticator-credential-properties-extension"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-credProtect-extension"

            given "the WebAuthn JSON dictionary for credProps and an enforced credential protection policy" {
                let json = serde_json::json!({
                    "credProps": true,
                    "credentialProtectionPolicy": "userVerificationRequired",
                    "enforceCredentialProtectionPolicy": true
                });
            }
            when "the extension dictionary is decoded and encoded again" {
                let inputs = AuthenticationExtensionsClientInputs::from_json_value(json.clone()).unwrap();
                let encoded = inputs.to_json_value().unwrap();
            }
            then "each extension input retains its specified JSON name and value" {
                assert_eq!(inputs.credential_properties(), Some(true));
                assert_eq!(
                    inputs.credential_protection_policy(),
                    Some(&CredentialProtectionPolicy::UserVerificationRequired)
                );
                assert_eq!(inputs.enforce_credential_protection_policy(), Some(true));
                assert_eq!(encoded, json);
            }
        }
}
