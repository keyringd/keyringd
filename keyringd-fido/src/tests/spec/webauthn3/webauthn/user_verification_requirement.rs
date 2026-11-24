use crate::webauthn::*;

keyringd_testkit::scenario! {
        case "Exposes every user-verification requirement value" {
            spec "https://www.w3.org/TR/webauthn-3/#enum-userVerificationRequirement"

            verify "the standardized values have their exact WebAuthn spellings" {
                assert_eq!(UserVerificationRequirement::Required.as_webauthn_str(), "required");
                assert_eq!(UserVerificationRequirement::Preferred.as_webauthn_str(), "preferred");
                assert_eq!(UserVerificationRequirement::Discouraged.as_webauthn_str(), "discouraged");
            }
        }

        case "Keep an unknown DOMString value" {
            spec "https://www.w3.org/TR/webauthn-3/#enum-userVerificationRequirement"

            verify "Keep an unknown DOMString value" {
                let value = UserVerificationRequirement::new("future-uv");
                assert_eq!(value.as_webauthn_str(), "future-uv");
                }

        }
}
