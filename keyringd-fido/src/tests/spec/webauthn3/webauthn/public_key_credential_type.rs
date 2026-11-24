use crate::webauthn::*;

keyringd_testkit::scenario! {
        case "Keep an unknown DOMString value" {
            spec "https://www.w3.org/TR/webauthn-3/#enum-credentialType"

            verify "Keep an unknown DOMString value" {
                let value: PublicKeyCredentialType = serde_json::from_str("\"future-key\"").unwrap();
                assert_eq!(value, PublicKeyCredentialType::Unknown("future-key".into()));
                assert_eq!(serde_json::to_string(&value).unwrap(), "\"future-key\"");
                }

        }
}
