use crate::CodecError;
use crate::codec::CborCodec;
use crate::codec::CborItem as Value;
use crate::codec::CborSimpleValue;

keyringd_testkit::scenario! {
        case "Reject subsequent CBOR items" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#message-encoding"

            verify "Reject subsequent CBOR items" {
                assert!(CborCodec::decode(&[0xa0, 0xa0]).is_err());
                }

        }
        case "Rejects non-minimum integers for normal format decoding" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#message-encoding"

            verify "Rejects non-minimum integers for normal format decoding" {
                assert!(CborCodec::decode(&[0x18, 0x00]).is_err());
                }

        }
    group "CborCodec::decode" {
            case "Enforces the shortest integer encoding at every CTAP boundary" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#ctap2-canonical-cbor-encoding-form"

                given "minimal and overlong encodings at every unsigned and negative integer-width boundary" {
                    let encodings: &[(&[u8], &[u8])] = &[
                        (&[0x17], &[0x18, 0x17]),
                        (&[0x18, 0x18], &[0x19, 0x00, 0x18]),
                        (&[0x18, 0xff], &[0x19, 0x00, 0xff]),
                        (&[0x19, 0x01, 0x00], &[0x1a, 0x00, 0x00, 0x01, 0x00]),
                        (&[0x19, 0xff, 0xff], &[0x1a, 0x00, 0x00, 0xff, 0xff]),
                        (
                            &[0x1a, 0x00, 0x01, 0x00, 0x00],
                            &[0x1b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00],
                        ),
                        (
                            &[0x1a, 0xff, 0xff, 0xff, 0xff],
                            &[0x1b, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff],
                        ),
                        (&[0x37], &[0x38, 0x17]),
                        (&[0x38, 0x18], &[0x39, 0x00, 0x18]),
                        (&[0x38, 0xff], &[0x39, 0x00, 0xff]),
                        (&[0x39, 0x01, 0x00], &[0x3a, 0x00, 0x00, 0x01, 0x00]),
                        (&[0x39, 0xff, 0xff], &[0x3a, 0x00, 0x00, 0xff, 0xff]),
                        (
                            &[0x3a, 0x00, 0x01, 0x00, 0x00],
                            &[0x3b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00],
                        ),
                        (
                            &[0x3a, 0xff, 0xff, 0xff, 0xff],
                            &[0x3b, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff],
                        ),
                    ];
                }
                when "each encoding is decoded as CTAP canonical CBOR" {
                    let results = encodings
                        .iter()
                        .map(|(minimal, overlong)| {
                            (
                                CborCodec::decode(minimal),
                                CborCodec::decode(overlong),
                            )
                        })
                        .collect::<Vec<_>>();
                }
                then "every minimal encoding is accepted and every overlong encoding is rejected" {
                    assert!(results
                        .iter()
                        .all(|(minimal, overlong)| minimal.is_ok() && overlong.is_err()));
                }
            }
            case "Rejects non-minimal and indefinite container lengths" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#ctap2-canonical-cbor-encoding-form"

                given "overlong empty byte strings, text strings, arrays, and maps plus every indefinite-length container form" {
                    let overlong = [
                        vec![0x58, 0x00],
                        vec![0x78, 0x00],
                        vec![0x98, 0x00],
                        vec![0xb8, 0x00],
                    ];
                    let indefinite = [
                        vec![0x5f, 0xff],
                        vec![0x7f, 0xff],
                        vec![0x9f, 0xff],
                        vec![0xbf, 0xff],
                    ];
                }
                when "every value is decoded as CTAP canonical CBOR" {
                    let results = overlong
                        .iter()
                        .chain(&indefinite)
                        .map(|encoded| CborCodec::decode(encoded))
                        .collect::<Vec<_>>();
                }
                then "all non-minimal and indefinite forms are rejected" {
                    assert!(results.into_iter().all(|result| result.is_err()));
                }
            }
            case "Requires CTAP maps to be ordered from the lowest key to the highest" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#ctap2-canonical-cbor-encoding-form"

                given "equivalent maps with ascending and descending unsigned-integer keys" {
                    let ascending = vec![0xa2, 0x01, 0x00, 0x02, 0x00];
                    let descending = vec![0xa2, 0x02, 0x00, 0x01, 0x00];
                }
                when "both maps are decoded as CTAP canonical CBOR" {
                    let ascending_result = CborCodec::decode(&ascending);
                    let descending_result = CborCodec::decode(&descending);
                }
                then "only the map whose keys are in ascending order is accepted" {
                    assert!(ascending_result.is_ok());
                    assert!(descending_result.is_err());
                }
            }
            case "Orders map keys by major type, encoded length, and byte-wise value" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#ctap2-canonical-cbor-encoding-form"

                given "a map containing unsorted unsigned, negative, byte-string, text-string, and simple-value keys" {
                    let value = Value::Map(vec![
                        (Value::Simple(CborSimpleValue::FalseValue), Value::Unsigned(4)),
                        (Value::TextString("a".to_owned()), Value::Unsigned(3)),
                        (Value::ByteString(vec![0]), Value::Unsigned(2)),
                        (Value::Negative(-1), Value::Unsigned(1)),
                        (Value::Unsigned(0), Value::Unsigned(0)),
                    ]);
                }
                when "the map is encoded in CTAP canonical form" {
                    let encoded = CborCodec::encode(value).unwrap();
                }
                then "the keys are sorted first by major type and then by their canonical bytes" {
                    assert_eq!(
                        encoded,
                        [
                            0xa5, 0x00, 0x00, 0x20, 0x01, 0x41, 0x00, 0x02, 0x61, b'a', 0x03,
                            0xf4, 0x04,
                        ]
                    );
                }
            }
            case "Rejects duplicate keys in nested CTAP maps" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#message-encoding"

                given "canonical CBOR containing two equal keys in a nested map" {
                    let duplicate_nested_key = vec![
                        0xa1, 0x01, // outer map: key 1
                        0xa2, 0x02, 0x00, 0x02, 0x01, // nested map: key 2 twice
                    ];
                }
                when "the value is decoded as CTAP canonical CBOR" {
                    let result = CborCodec::decode(&duplicate_nested_key);
                }
                then "decoding rejects the duplicate key at the nested map boundary" {
                    assert!(result.is_err());
                }
            }
            case "Rejects CBOR tags because CTAP messages do not permit them" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#message-encoding"

                given "a canonically encoded CBOR item wrapped in tag zero" {
                    let tagged = vec![0xc0, 0x00];
                }
                when "the item is decoded as CTAP canonical CBOR" {
                    let result = CborCodec::decode(&tagged);
                }
                then "decoding fails instead of exposing the tagged value to protocol parsers" {
                    assert!(matches!(result, Err(CodecError::InvalidCbor(message)) if message.to_string().contains("tags")));
                }

            }
            case "Accepts four container levels and rejects a fifth level" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#message-encoding"

                given "canonical arrays nested to the CTAP limit and one level beyond it" {
                    let four_levels = vec![0x81, 0x81, 0x81, 0x81, 0x00];
                    let five_levels = vec![0x81, 0x81, 0x81, 0x81, 0x81, 0x00];
                }
                when "both values are decoded as CTAP canonical CBOR" {
                    let accepted = CborCodec::decode(&four_levels);
                    let rejected = CborCodec::decode(&five_levels);
                }
                then "only the value within the four-level nesting limit is accepted" {
                    assert!(accepted.is_ok());
                    assert!(matches!(rejected, Err(CodecError::InvalidCbor(message)) if message.to_string().contains("four levels")));
                }

            }
            case "Preserves the encoded width of every floating-point representation" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#ctap2-canonical-cbor-encoding-form"

                given "the value 1.5 encoded separately as 16-bit, 32-bit, and 64-bit CBOR floating point" {
                    let encodings = [
                        vec![0xf9, 0x3e, 0x00],
                        vec![0xfa, 0x3f, 0xc0, 0x00, 0x00],
                        vec![0xfb, 0x3f, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                    ];
                }
                when "each representation is decoded and encoded again" {
                    let round_trips = encodings
                        .iter()
                        .map(|encoded| {
                            CborCodec::encode(CborCodec::decode(encoded).unwrap()).unwrap()
                        })
                        .collect::<Vec<_>>();
                }
                then "each result retains its original width because CTAP treats width as semantic" {
                    assert_eq!(round_trips, encodings);
                }
            }
    }
    group "CborCodec::encode" {
            case "Sorts CTAP map keys from the lowest value to the highest" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#ctap2-canonical-cbor-encoding-form"

                given "a CBOR map value whose unsigned-integer keys are supplied in descending order" {
                    let value = Value::Map(vec![
                        (Value::Unsigned(2), Value::Unsigned(0)),
                        (Value::Unsigned(1), Value::Unsigned(0)),
                    ]);
                }
                when "the map is encoded for the CTAP wire" {
                    let encoded = CborCodec::encode(value).unwrap();
                }
                then "the emitted map contains the key-value pairs in ascending key order" {
                    assert_eq!(encoded, [0xa2, 0x01, 0x00, 0x02, 0x00]);
                }
            }
            case "Rejects duplicate keys before encoding a CTAP map" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#message-encoding"

                given "a CBOR value containing two entries with the same map key" {
                    let duplicate_keys = Value::Map(vec![
                        (Value::Unsigned(1), Value::Unsigned(0)),
                        (Value::Unsigned(1), Value::Unsigned(1)),
                    ]);
                }
                when "the value is passed to the CTAP encoder" {
                    let result = CborCodec::encode(duplicate_keys);
                }
                then "encoding rejects the map instead of emitting duplicate keys" {
                    assert!(result.is_err());
                }
            }
            case "Rejects CTAP-forbidden CBOR structures before encoding" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#message-encoding"

                given "a tagged value and a value nested five container levels deep" {
                    let tagged = Value::Tag(0, Box::new(Value::Unsigned(0)));
                    let five_levels = (0..5).fold(Value::Unsigned(0), |value, _| {
                        Value::Array(vec![value])
                    });
                }
                when "both values are passed to the CTAP encoder" {
                    let tagged_result = CborCodec::encode(tagged);
                    let nested_result = CborCodec::encode(five_levels);
                }
                then "neither forbidden structure can be emitted onto the CTAP wire" {
                    assert!(matches!(tagged_result, Err(CodecError::InvalidCbor(message)) if message.to_string().contains("tags")));
                    assert!(matches!(nested_result, Err(CodecError::InvalidCbor(message)) if message.to_string().contains("four levels")));
                }
            }
    }
    group "CborCodec::encoded_item_length" {
            case "Rejects an indeterminate or cut item in the middle" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#message-encoding"

                verify "Rejects an indeterminate or cut item in the middle" {
                    assert!(CborCodec::encoded_item_length(&[0x9f, 0x01, 0xff]).is_err());
                    assert!(CborCodec::encoded_item_length(&[0x58, 0x20, 0x01]).is_err());
                    }

            }
    }
}
