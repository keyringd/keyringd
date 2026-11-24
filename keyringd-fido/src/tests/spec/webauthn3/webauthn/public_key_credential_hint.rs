use crate::webauthn::*;

keyringd_testkit::scenario! {
        case "Uses the standardized WebAuthn credential hints and preserves future values" {
            spec "https://www.w3.org/TR/webauthn-3/#enum-hints"

            given "the standardized hints and a value introduced by a future specification" {
                let hints = [
                    PublicKeyCredentialHint::security_key(),
                    PublicKeyCredentialHint::client_device(),
                    PublicKeyCredentialHint::hybrid(),
                    PublicKeyCredentialHint::new("future-hint"),
                ];
            }
            when "their DOMString representations are read" {
                let values = hints.map(|hint| hint.as_str().to_owned());
            }
            then "the standard spellings and unknown value are retained exactly" {
                assert_eq!(
                    values,
                    ["security-key", "client-device", "hybrid", "future-hint"]
                );
            }
        }
}
