use crate::ProtocolError;
use crate::codec::CborCodec;
use crate::codec::CborItem as Value;
use crate::ctap::ALGORITHM_ECDH_ES_HKDF_256;
use crate::ctap::CURVE_P256;
use crate::ctap::KEY_TYPE_EC2;
use crate::ctap::PinUvKeyAgreementKey;

keyringd_testkit::scenario! {
    group "PinUvKeyAgreementKey::from_cbor_bytes" {
        case "Rejects a key-agreement key with the wrong COSE algorithm" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorClientPIN"

            given "a PIN/UV key-agreement COSE key that declares ES256 instead of ECDH-ES with HKDF-256" {
                let encoded = CborCodec::encode(Value::Map(vec![
                        (Value::integer(1), Value::integer(KEY_TYPE_EC2)),
                        (Value::integer(3), Value::integer(-7)),
                        (Value::integer(-1), Value::integer(CURVE_P256)),
                        (Value::integer(-2), Value::ByteString(vec![1; 32])),
                        (Value::integer(-3), Value::ByteString(vec![2; 32])),
                    ])).unwrap();
            }
            when "the COSE key is decoded as a PIN/UV key-agreement key" {
                let result = PinUvKeyAgreementKey::from_cbor_bytes(&encoded);
            }
            then "decoding rejects the algorithm that is not permitted for PIN/UV key agreement" {
                assert!(result.is_err());
            }
        }
        case "Requires the alg parameter in a PIN/UV key-agreement key" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorclientpin-rpid"

            given "an otherwise complete PIN/UV key-agreement COSE key without the alg parameter" {
                let encoded = CborCodec::encode(Value::Map(vec![
                    (Value::integer(1), Value::integer(KEY_TYPE_EC2)),
                    (Value::integer(-1), Value::integer(CURVE_P256)),
                    (Value::integer(-2), Value::ByteString(vec![1; 32])),
                    (Value::integer(-3), Value::ByteString(vec![2; 32])),
                ]))
                .unwrap();
            }
            when "the COSE key is decoded as a PIN/UV key-agreement key" {
                let result = PinUvKeyAgreementKey::from_cbor_bytes(&encoded);
            }
            then "decoding rejects the key because authenticatorClientPIN requires alg" {
                assert!(result.is_err());
            }
        }
        case "Rejects an optional COSE key parameter other than alg" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorclientpin-rpid"

            given "a PIN/UV key-agreement key with an additional key_ops parameter" {
                let encoded = CborCodec::encode(Value::Map(vec![
                    (Value::integer(1), Value::integer(KEY_TYPE_EC2)),
                    (
                        Value::integer(3),
                        Value::integer(ALGORITHM_ECDH_ES_HKDF_256),
                    ),
                    (Value::integer(4), Value::Array(vec![Value::integer(-7)])),
                    (Value::integer(-1), Value::integer(CURVE_P256)),
                    (Value::integer(-2), Value::ByteString(vec![1; 32])),
                    (Value::integer(-3), Value::ByteString(vec![2; 32])),
                ]))
                .unwrap();
            }
            when "the COSE key is decoded as a PIN/UV key-agreement key" {
                let result = PinUvKeyAgreementKey::from_cbor_bytes(&encoded);
            }
            then "decoding rejects the optional parameter forbidden by authenticatorClientPIN" {
                assert!(matches!(
                    result,
                    Err(ProtocolError::InvalidValue {
                        field: "keyAgreement",
                        ..
                    })
                ));
            }
        }
    }
}

keyringd_testkit::scenario! {
        case "Can be converted back and forth between constrained keys" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorClientPIN"

            given "Prepare SEC1" {
                let mut sec1 = [0; 65];
                sec1[0] = 4;
                sec1[1] = 1;
                sec1[33] = 2;
            }
            when "Call PinUvKeyAgreementKey::from_sec1_public_key" {
                let key = PinUvKeyAgreementKey::from_sec1_public_key(&sec1).unwrap();
            }
            then "Can be converted back and forth between constrained keys" {
                assert_eq!(key.to_sec1_public_key(), sec1);
                assert_eq!(
                    PinUvKeyAgreementKey::from_cbor_bytes(&key.to_cbor_bytes().unwrap()).unwrap(),
                    key
                );
            }

        }
}
