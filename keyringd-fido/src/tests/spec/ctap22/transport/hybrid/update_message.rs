use crate::EncodedCborValue;
use crate::codec::CborItem as Value;
use crate::transport::hybrid::HybridUpdateMessage;
use crate::transport::hybrid::codec::encode_map;

keyringd_testkit::scenario! {
        case "Verify Padding by Converting Known Fields" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-hybrid"

            given "Prepare a HybridUpdateMessage, EncodedCborValue" {
                let value = HybridUpdateMessage {
                    padding: Some(vec![0; 2]),
                    linking_information: Some(
                        EncodedCborValue::from_canonical_bytes(vec![0xa1, 0x01, 0x41, 0x02]).unwrap(),
                    ),
                };
            }
            when "Call value.to_cbor_bytes" {
                let encoded = value.to_cbor_bytes().unwrap();
            }
            then "Verify Padding by Converting Known Fields" {
                assert_eq!(
                    HybridUpdateMessage::from_cbor_bytes(&encoded).unwrap(),
                    value
                );
                let mut invalid = value;
                invalid.padding = Some(vec![0, 1]);
                assert!(invalid.to_cbor_bytes().is_err());
            }

        }

        case "Ignore the Unknown Key of a Normal CBOR" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-hybrid"

            given "Prepare a regular CBOR map with known padding and unknown key 9" {
                let encoded = encode_map(vec![
                    (0, Value::ByteString(vec![0, 0])),
                    (9, Value::TextString("future field".to_owned())),
                ]).unwrap();
            }
            when "Decode as HybridUpdateMessage" {
                let decoded = HybridUpdateMessage::from_cbor_bytes(&encoded).unwrap();
            }
            then "The unknown key is ignored and only known FIELDs are retained" {
                assert_eq!(decoded.padding(), Some([0, 0].as_slice()));
                assert_eq!(decoded.linking_information(), None);
                assert_eq!(
                    decoded.to_cbor_bytes().unwrap(),
                    encode_map(vec![(0, Value::ByteString(vec![0, 0]))]).unwrap()
                );
            }
        }
}
