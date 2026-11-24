use crate::webauthn::*;

keyringd_testkit::scenario! {
        case "Uses the registered COSE curve number for P-256" {
            spec "https://www.rfc-editor.org/rfc/rfc9053.html#section-2.1"

            verify "P-256 is represented by curve value one" {
                assert_eq!(CoseEllipticCurve::P256.value(), 1);
            }
        }
}
