use crate::webauthn::*;

keyringd_testkit::scenario! {
        case "Exposes every resident-key requirement value" {
            spec "https://www.w3.org/TR/webauthn-3/#enum-residentKeyRequirement"

            verify "the standardized values have their exact WebAuthn spellings" {
                assert_eq!(ResidentKeyRequirement::Discouraged.as_str(), "discouraged");
                assert_eq!(ResidentKeyRequirement::Preferred.as_str(), "preferred");
                assert_eq!(ResidentKeyRequirement::Required.as_str(), "required");
            }
        }

        case "Keep an unknown DOMString value" {
            spec "https://www.w3.org/TR/webauthn-3/#enum-residentKeyRequirement"

            verify "Keep an unknown DOMString value" {
                let value = ResidentKeyRequirement::new("future-resident-key");
                assert_eq!(value.as_str(), "future-resident-key");
                }

        }
}
