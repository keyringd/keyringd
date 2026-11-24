use crate::ProtocolError;
use crate::transport::hybrid::HybridOperationHint;
use crate::transport::hybrid::HybridQrPayload;

keyringd_testkit::scenario! {
        case "Encode Operation Hint as a CBOR text string" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

            given "Operation Hint prepares GA Hybrid QR Payload" {
                let value = HybridQrPayload {
                    public_key: [2; 33],
                    qr_secret: [3; 16],
                    known_tunnel_server_domains: 2,
                    current_time: Some(42),
                    can_use_state_assisted_transactions: None,
                    operation_hint: HybridOperationHint::GetAssertion,
                };
            }
            when "Encode Payload to CBOR" {
                let encoded = value.to_cbor_bytes().unwrap();
            }
            then "Operation Hints are stored as CBOR Text Strings" {
                assert!(encoded.ends_with(&[5, 0x62, b'g', b'a']));
            }
        }
    group "HybridQrPayload::from_cbor_bytes" {
        case "Treats an unknown operation-hint value as GetAssertion" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

            given "a canonical hybrid QR payload whose two-byte operation hint is unknown" {
                let value = HybridQrPayload::new(
                    [2; 33],
                    [3; 16],
                    2,
                    HybridOperationHint::GetAssertion,
                );
                let mut encoded = value.to_cbor_bytes().unwrap();
                let hint = encoded.len() - 2;
                encoded[hint..].copy_from_slice(b"zz");
            }
            when "the payload is decoded" {
                let decoded = HybridQrPayload::from_cbor_bytes(&encoded).unwrap();
            }
            then "the unknown hint is interpreted as the ga GetAssertion flow" {
                assert_eq!(decoded.operation_hint(), HybridOperationHint::GetAssertion);
            }
        }
    }
    group "HybridQrPayload::from_cbor_bytes" {
            case "Round-trip conversion of all fields in normal format" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

                given "Prepare a HybridQrPayLoad" {
                    let value = HybridQrPayload {
                        public_key: [2; 33],
                        qr_secret: [3; 16],
                        known_tunnel_server_domains: 2,
                        current_time: Some(42),
                        can_use_state_assisted_transactions: Some(false),
                        operation_hint: HybridOperationHint::GetAssertion,
                    };
                }
                when "Call value.to_cbor_bytes" {
                    let encoded = value.to_cbor_bytes().unwrap();
                }
                then "QR Payload after DECODE matches all fields before Encode" {
                    assert_eq!(HybridQrPayload::from_cbor_bytes(&encoded).unwrap(), value);
                }

            }
            case "Rejects Operation Hint in Byte String" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

                given "Prepare a HybridQrPayLoad" {
                    let value = HybridQrPayload {
                        public_key: [2; 33],
                        qr_secret: [3; 16],
                        known_tunnel_server_domains: 2,
                        current_time: None,
                        can_use_state_assisted_transactions: None,
                        operation_hint: HybridOperationHint::GetAssertion,
                    };
                    let mut encoded = value.to_cbor_bytes().unwrap();
                }
                when "Rewrite Operation Hint's CBOR Header to a Byte String" {
                    let hint_header = encoded.len() - 3;
                    encoded[hint_header] = 0x42;
                }
                then "Verify that the results match ProtocolError::UnexpectedType" {
                    assert!(matches!(
                        HybridQrPayload::from_cbor_bytes(&encoded),
                        Err(ProtocolError::UnexpectedType {
                            field: "hybrid QR operation hint",
                            expected: "a text string",
                        })
                    ));
                }

            }
            case "Reject Missing and Non-Normal Formats of Required Fields" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

                given "Prepare a Payload Missing Required Field and a Non-Canonical Payload" {
                    let missing_fields = [0xa0];
                    let value = HybridQrPayload {
                        public_key: [2; 33],
                        qr_secret: [3; 16],
                        known_tunnel_server_domains: 2,
                        current_time: None,
                        can_use_state_assisted_transactions: None,
                        operation_hint: HybridOperationHint::GetAssertion,
                    };
                    let mut noncanonical = value.to_cbor_bytes().unwrap();
                    noncanonical[1] = 0x18;
                    noncanonical.insert(2, 0);
                }
                when "Decode both payloads" {
                    let missing_result = HybridQrPayload::from_cbor_bytes(&missing_fields);
                    let noncanonical_result = HybridQrPayload::from_cbor_bytes(&noncanonical);
                }
                then "Both missing and non-canonical expressions are rejected" {
                    assert!(missing_result.is_err());
                    assert!(noncanonical_result.is_err());
                }

            }
    }
}
