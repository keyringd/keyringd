use crate::codec::CborItem as Value;
use crate::transport::hybrid::HybridPostHandshakeMessage;
use crate::transport::hybrid::codec::encode_map;

keyringd_testkit::scenario! {
    group "HybridPostHandshakeMessage::from_cbor_bytes" {
        case "Defaults an omitted supported-feature list to CTAP" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

            given "a canonical hybrid post-handshake message containing GetInfo but no key 3 feature list" {
                let encoded = encode_map(vec![(1, Value::ByteString(vec![0xa0]))]).unwrap();
            }
            when "the post-handshake message is decoded" {
                let decoded = HybridPostHandshakeMessage::from_cbor_bytes(&encoded).unwrap();
            }
            then "the omitted feature list is interpreted as containing CTAP" {
                assert_eq!(decoded.supported_features(), ["ctap"]);
            }
        }
        case "Rejects a non-canonical post-handshake map" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

            given "Add unnecessarily add key 1 Prepare a Post-Handshake Map represented by Byte" {
                let encoded = [0xa1, 0x18, 0x01, 0x41, 0xa0];
            }
            when "Decode as post-handshake message" {
                let result = HybridPostHandshakeMessage::from_cbor_bytes(&encoded);
            }
            then "Rejected as non-regular CBOR" {
                assert!(result.is_err());
            }
        }
    }
    group "HybridPostHandshakeMessage::to_cbor_bytes" {
            case "Force default Feature and Padding rules" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-hybrid"

                given "Prepare a HybridPostHandShakeMessage" {
                    let value = HybridPostHandshakeMessage {
                        padding: Some(vec![0; 3]),
                        get_info_response: vec![0xa0],
                        supported_features: vec!["ctap".to_owned()],
                    };
                }
                when "Call value.to_cbor_bytes" {
                    let encoded = value.to_cbor_bytes().unwrap();
                }
                then "Force default Feature and Padding rules" {
                    assert_eq!(
                        HybridPostHandshakeMessage::from_cbor_bytes(&encoded).unwrap(),
                        value
                    );
                    let mut invalid = value;
                    invalid.padding = Some(vec![1]);
                    assert!(invalid.to_cbor_bytes().is_err());
                }

            }
            case "Round-trips explicitly advertised features" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-hybrid"

                given "Prepare a HybridPostHandShakeMessage" {
                    let value = HybridPostHandshakeMessage {
                        padding: None,
                        get_info_response: vec![0xa0],
                        supported_features: vec!["ctap".to_owned(), "dc".to_owned()],
                    };
                }
                when "Call value.to_cbor_bytes" {
                    let encoded = value.to_cbor_bytes().unwrap();
                }
                then "The explicit feature set is retained even after the decode" {
                    assert_eq!(
                        HybridPostHandshakeMessage::from_cbor_bytes(&encoded).unwrap(),
                        value
                    );
                }

            }
    }
}
