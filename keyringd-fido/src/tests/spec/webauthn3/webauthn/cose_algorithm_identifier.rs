use crate::webauthn::*;

keyringd_testkit::scenario! {
        case "Uses the registered COSE algorithm number for ES256" {
            spec "https://www.rfc-editor.org/rfc/rfc9053.html#section-2.1"

            verify "ES256 is represented by the registered algorithm value negative seven" {
                assert_eq!(CoseAlgorithmIdentifier::ES256.value(), -7);
            }
        }
}
