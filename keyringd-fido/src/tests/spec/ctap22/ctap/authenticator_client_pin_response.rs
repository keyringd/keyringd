use crate::codec::CborCodec;
use crate::codec::CborItem as Value;
use crate::ctap::AuthenticatorClientPinResponse;
use crate::ctap::CtapValue;
use crate::ctap::PinUvKeyAgreementKey;

keyringd_testkit::scenario! {
        case "Round-trips every authenticatorClientPIN response field" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorClientPIN"

            given "a response containing every field defined by authenticatorClientPIN" {
                let mut sec1 = [2u8; 65];
                sec1[0] = 4;
                let response = AuthenticatorClientPinResponse::new()
                    .with_key_agreement(PinUvKeyAgreementKey::from_sec1_public_key(&sec1).unwrap())
                    .with_pin_uv_auth_token(vec![3; 32])
                    .with_pin_retries(8)
                    .with_power_cycle_state(false)
                    .with_uv_retries(5);
            }
            when "the response is encoded and decoded" {
                let encoded = response.to_cbor_bytes().unwrap();
                let decoded = AuthenticatorClientPinResponse::from_cbor_bytes(&encoded).unwrap();
                let fields = CtapValue::ctap_map(
                    CborCodec::decode(&encoded).unwrap(),
                )
                .unwrap();
                let key_agreement = CborCodec::decode(
                    &response.key_agreement().unwrap().to_cbor_bytes().unwrap(),
                )
                .unwrap();
            }
            then "all fields retain their specified wire values" {
                assert_eq!(decoded, response);
                assert_eq!(decoded.pin_retries(), Some(8));
                assert_eq!(decoded.power_cycle_state(), Some(false));
                assert_eq!(decoded.uv_retries(), Some(5));
                assert_eq!(
                    fields,
                    std::collections::BTreeMap::from([
                        (1, key_agreement),
                        (2, Value::ByteString(vec![3; 32])),
                        (3, Value::Unsigned(8)),
                        (4, Value::bool_value(false)),
                        (5, Value::Unsigned(5)),
                    ])
                );
            }

        }
}
