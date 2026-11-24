use crate::webauthn::*;

keyringd_testkit::scenario! {
    group "AttestationStatement::from_cbor_bytes" {
            case "Round-trip conversion of known attestation formats" {
                spec "https://www.w3.org/TR/webauthn-3/#sctn-attestation-formats"

                given "Prepare Attestation Statement for None, Packed and FIDO U2F" {
                    let values = [
                        AttestationStatement::None,
                        AttestationStatement::Packed(PackedAttestationStatement::new(
                            CoseAlgorithmIdentifier::ES256,
                            vec![1, 2],
                        )),
                        AttestationStatement::FidoU2f(FidoU2fAttestationStatement::new(vec![3, 4], vec![5, 6])),
                    ];
                }
                when "Encode each format to CBOR and then decode" {
                    let decoded = values.iter().map(|value| {
                        AttestationStatement::from_cbor_bytes(
                                value.format(),
                                &value.to_cbor_bytes().unwrap(),
                            )
                            .unwrap()
                    }).collect::<Vec<_>>();
                }
                then "All known formats are restored to their original values" {
                    assert_eq!(decoded, values);
                }

            }

            case "Matches attestation format identifiers case-sensitively" {
                spec "https://www.w3.org/TR/webauthn-3/#sctn-attstn-fmt-ids"

                verify "packed is recognized while differently-cased PACKED remains unknown" {
                    let encoded = AttestationStatement::Packed(PackedAttestationStatement::new(
                        CoseAlgorithmIdentifier::ES256,
                        vec![1],
                    ))
                    .to_cbor_bytes()
                    .unwrap();
                    assert!(matches!(
                        AttestationStatement::from_cbor_bytes("packed", &encoded).unwrap(),
                        AttestationStatement::Packed(_)
                    ));
                    assert!(matches!(
                        AttestationStatement::from_cbor_bytes("PACKED", &encoded).unwrap(),
                        AttestationStatement::Unknown { .. }
                    ));
                }
            }

            case "Rejects a non-empty none attestation statement" {
                spec "https://www.w3.org/TR/webauthn-3/#sctn-none-attestation"

                verify "none accepts only the empty CBOR map" {
                    assert!(matches!(
                        AttestationStatement::from_cbor_bytes("none", &[0xa1, 0x01, 0x01]),
                        Err(ProtocolError::UnexpectedType {
                            field: "none attStmt",
                            expected: "an empty map"
                        })
                    ));
                }
            }
    }
}
