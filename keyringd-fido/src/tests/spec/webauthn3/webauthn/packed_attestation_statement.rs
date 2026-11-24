use crate::ProtocolError;
use crate::codec::CborItem as Value;
use crate::tests::support::webauthn_packed_attestation_statement::statement;
use crate::webauthn::CoseAlgorithmIdentifier;
use crate::webauthn::PackedAttestationStatement;

keyringd_testkit::scenario! {
        case "Round-trips the packed algorithm signature and certificate chain in order" {
            spec "https://www.w3.org/TR/webauthn-3/#sctn-packed-attestation"

            verify "the attestation certificate remains first and all fields retain their bytes" {
                let statement = PackedAttestationStatement::new(
                    CoseAlgorithmIdentifier::ES256,
                    vec![1, 2, 3],
                )
                .with_certificate_chain(vec![vec![4], vec![5], vec![6]]);
                let decoded = PackedAttestationStatement::from_cbor_value(
                    statement.to_cbor_value().unwrap(),
                )
                .unwrap();
                assert_eq!(decoded.algorithm(), CoseAlgorithmIdentifier::ES256);
                assert_eq!(decoded.signature(), [1, 2, 3]);
                assert_eq!(decoded.certificate_chain().unwrap(), [vec![4], vec![5], vec![6]]);
            }
        }

        case "Reject an empty Certificate Chain when decoding and encoding" {
            spec "https://www.w3.org/TR/webauthn-3/#sctn-packed-attestation"

            given "Prepare a CBOR value with an empty X5C and a Packed Attestation" {
                let encoded_value = Value::Map(vec![
                    (Value::TextString("alg".into()), Value::Negative(-7)),
                    (Value::TextString("sig".into()), Value::ByteString(vec![1])),
                    (Value::TextString("x5c".into()), Value::Array(vec![])),
                ]);
                let mut statement = statement();
                statement.certificate_chain = Some(vec![]);
            }
            when "Run both decode and encode directions" {
                let decode_result = PackedAttestationStatement::from_cbor_value(encoded_value);
                let encode_result = statement.to_cbor_value();
            }
            then "Both directions are rejected as empty certificate chains" {
                assert!(matches!(
                    decode_result,
                    Err(ProtocolError::EmptyValue {
                        field: "packed.x5c"
                    })
                ));
                assert!(matches!(
                    encode_result,
                    Err(ProtocolError::EmptyValue {
                        field: "packed.x5c"
                    })
                ));
            }

        }

        case "Enforce required fields and CBOR types in Packed Attestation" {
            spec "https://www.w3.org/TR/webauthn-3/#sctn-packed-attestation"

            given "Missing ALG or SIG and each field has a wrong CBOR type." {
                let values = [
                    Value::Map(vec![(
                        Value::TextString("sig".into()),
                        Value::ByteString(vec![1]),
                    )]),
                    Value::Map(vec![
                        (Value::TextString("alg".into()), Value::TextString("-7".into())),
                        (Value::TextString("sig".into()), Value::ByteString(vec![1])),
                    ]),
                    Value::Map(vec![(Value::TextString("alg".into()), Value::Negative(-7))]),
                    Value::Map(vec![
                        (Value::TextString("alg".into()), Value::Negative(-7)),
                        (Value::TextString("sig".into()), Value::TextString("sig".into())),
                    ]),
                    Value::Map(vec![
                        (Value::TextString("alg".into()), Value::Negative(-7)),
                        (Value::TextString("sig".into()), Value::ByteString(vec![1])),
                        (Value::TextString("x5c".into()), Value::bool_value(true)),
                    ]),
                    Value::Map(vec![
                        (Value::TextString("alg".into()), Value::Negative(-7)),
                        (Value::TextString("sig".into()), Value::ByteString(vec![1])),
                        (
                            Value::TextString("x5c".into()),
                            Value::Array(vec![Value::TextString("certificate".into())]),
                        ),
                    ]),
                ];
            }
            when "Decode each MAP as a Packed Attestation" {
                let results = values.map(PackedAttestationStatement::from_cbor_value);
            }
            then "Reject all cddl required and non-conforming values" {
                assert!(results.into_iter().all(|result| result.is_err()));
            }
        }

}
