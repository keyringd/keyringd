use crate::webauthn::*;

keyringd_testkit::scenario! {
        case "Exposes the platform and cross-platform attachment values" {
            spec "https://www.w3.org/TR/webauthn-3/#enum-attachment"

            verify "the standardized values have their exact WebAuthn spellings" {
                assert_eq!(AuthenticatorAttachment::Platform.as_str(), "platform");
                assert_eq!(AuthenticatorAttachment::CrossPlatform.as_str(), "cross-platform");
            }
        }

        case "Keep an unknown DOMString value" {
            spec "https://www.w3.org/TR/webauthn-3/#enum-attachment"

            verify "Keep an unknown DOMString value" {
                let value = AuthenticatorAttachment::new("future-attachment");
                assert_eq!(value.as_str(), "future-attachment");
                }

        }
}
