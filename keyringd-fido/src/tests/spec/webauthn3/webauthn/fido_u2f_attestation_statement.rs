use crate::codec::CborItem as Value;
use crate::webauthn::FidoU2fAttestationStatement;

keyringd_testkit::scenario! {
        case "Requires exactly one Attestation Certificate" {
            spec "https://www.w3.org/TR/webauthn-3/#sctn-fido-u2f-attestation"

            given "Prepare 0 or 2 FIDO U2F Attestation CBORs with Certificate" {
                let values = [vec![], vec![vec![1], vec![2]]].map(|certificates| {
                    Value::Map(vec![
                        (
                            Value::TextString("x5c".into()),
                            Value::Array(certificates.into_iter().map(Value::ByteString).collect()),
                        ),
                        (Value::TextString("sig".into()), Value::ByteString(vec![3])),
                    ])
                });
            }
            when "Decode each CBOR value as FIDO U2F Attestation" {
                let results = values.map(FidoU2fAttestationStatement::from_cbor_value);
            }
            then "Any value that is not exactly one is rejected" {
                assert!(results.into_iter().all(|result| result.is_err()));
            }

        }

        case "Enforce required fields and CBOR types of FIDO U2F Attestation" {
            spec "https://www.w3.org/TR/webauthn-3/#sctn-fido-u2f-attestation"

            given "Missing X5C or SIG, each field has a wrong CBOR type." {
                let values = [
                    Value::Map(vec![(
                        Value::TextString("sig".into()),
                        Value::ByteString(vec![1]),
                    )]),
                    Value::Map(vec![(
                        Value::TextString("x5c".into()),
                        Value::Array(vec![Value::ByteString(vec![1])]),
                    )]),
                    Value::Map(vec![
                        (Value::TextString("x5c".into()), Value::bool_value(true)),
                        (Value::TextString("sig".into()), Value::ByteString(vec![1])),
                    ]),
                    Value::Map(vec![
                        (
                            Value::TextString("x5c".into()),
                            Value::Array(vec![Value::TextString("certificate".into())]),
                        ),
                        (Value::TextString("sig".into()), Value::ByteString(vec![1])),
                    ]),
                    Value::Map(vec![
                        (
                            Value::TextString("x5c".into()),
                            Value::Array(vec![Value::ByteString(vec![1])]),
                        ),
                        (Value::TextString("sig".into()), Value::TextString("sig".into())),
                    ]),
                ];
            }
            when "Decode each MAP as FIDO U2F Attestation" {
                let results = values.map(FidoU2fAttestationStatement::from_cbor_value);
            }
            then "Reject all cddl required and non-conforming values" {
                assert!(results.into_iter().all(|result| result.is_err()));
            }
        }

}
