use crate::transport::hybrid::HybridOperationHint;

keyringd_testkit::scenario! {
    group "HybridOperationHint wire conversion" {
        case "Uses the hybrid operation abbreviations defined for QR transactions" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

            given "the assertion and credential-creation operation hints" {
                let hints = [
                    HybridOperationHint::GetAssertion,
                    HybridOperationHint::MakeCredential,
                ];
            }
            when "their hybrid URL representations are obtained" {
                let values = hints.map(HybridOperationHint::as_str);
            }
            then "the hints use ga and mc respectively" {
                assert_eq!(values, ["ga", "mc"]);
            }
        }
        case "Treats an unknown hybrid operation hint as GetAssertion" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

            given "an operation-hint value that is not defined by CTAP 2.2" {
                let wire_value = "future-operation";
            }
            when "the operation hint is interpreted" {
                let hint = HybridOperationHint::from_wire_value(wire_value);
            }
            then "the hint is treated as the GetAssertion hint ga" {
                assert_eq!(hint, HybridOperationHint::GetAssertion);
                assert_eq!(hint.as_str(), "ga");
            }
        }
    }
}
