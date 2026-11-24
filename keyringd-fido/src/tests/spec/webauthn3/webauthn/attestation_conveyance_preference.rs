use crate::webauthn::*;

keyringd_testkit::scenario! {
        case "Exposes every attestation conveyance preference" {
            spec "https://www.w3.org/TR/webauthn-3/#enum-attestation-convey"

            verify "the standardized values have their exact WebAuthn spellings" {
                assert_eq!(AttestationConveyancePreference::None.as_str(), "none");
                assert_eq!(AttestationConveyancePreference::Indirect.as_str(), "indirect");
                assert_eq!(AttestationConveyancePreference::Direct.as_str(), "direct");
                assert_eq!(AttestationConveyancePreference::Enterprise.as_str(), "enterprise");
            }
        }

        case "Keep an unknown DOMString value" {
            spec "https://www.w3.org/TR/webauthn-3/#enum-attestation-convey"

            verify "Keep an unknown DOMString value" {
                let value = AttestationConveyancePreference::new("future-attestation");
                assert_eq!(value.as_str(), "future-attestation");
                assert_eq!(
                    value,
                    AttestationConveyancePreference::Unknown("future-attestation".to_owned())
                );
                }

        }
}
