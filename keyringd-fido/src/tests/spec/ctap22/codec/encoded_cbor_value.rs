use crate::EncodedCborValue;

keyringd_testkit::scenario! {
        case "Accepts one canonical CBOR item and preserves its exact representation" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#message-encoding"

            given "a canonical integer, a width-sensitive floating-point value, a non-minimal integer, and concatenated values" {
                let canonical = vec![0x18, 0x2a];
                let float16 = vec![0xf9, 0x3e, 0x00];
                let nonminimal = vec![0x18, 0x00];
                let concatenated = vec![0x00, 0x00];
            }
            when "each byte sequence is interpreted as one opaque canonical CBOR value" {
                let canonical_result = EncodedCborValue::from_canonical_bytes(canonical);
                let float_result = EncodedCborValue::from_canonical_bytes(float16);
                let nonminimal_result = EncodedCborValue::from_canonical_bytes(nonminimal);
                let concatenated_result = EncodedCborValue::from_canonical_bytes(concatenated);
            }
            then "canonical items retain their bytes while non-canonical or concatenated input is rejected" {
                assert_eq!(
                    canonical_result.unwrap().as_bytes(),
                    [0x18, 0x2a]
                );
                assert_eq!(float_result.unwrap().as_bytes(), [0xf9, 0x3e, 0x00]);
                assert!(nonminimal_result.is_err());
                assert!(concatenated_result.is_err());
            }

        }
}
