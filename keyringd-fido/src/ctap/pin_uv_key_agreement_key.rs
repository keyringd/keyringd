use crate::codec::CborItem as Value;

use crate::ProtocolError;
use crate::codec::CborCodec;

pub(crate) const KEY_TYPE_EC2: i64 = 2;
pub(crate) const ALGORITHM_ECDH_ES_HKDF_256: i64 = -25;
pub(crate) const CURVE_P256: i64 = 1;

/// The restricted COSE_Key representation used by the CTAP PIN/UV protocols.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PinUvKeyAgreementKey {
    x: [u8; 32],
    y: [u8; 32],
}

impl PinUvKeyAgreementKey {
    pub fn from_sec1_public_key(key: &[u8; 65]) -> Result<Self, ProtocolError> {
        if key[0] != 0x04 {
            return Err(ProtocolError::UnexpectedType {
                field: "keyAgreement",
                expected: "an uncompressed P-256 SEC1 public key",
            });
        }
        let mut x = [0; 32];
        let mut y = [0; 32];
        x.copy_from_slice(&key[1..33]);
        y.copy_from_slice(&key[33..65]);
        Ok(Self { x, y })
    }

    pub fn to_sec1_public_key(&self) -> [u8; 65] {
        let mut key = [0; 65];
        key[0] = 0x04;
        key[1..33].copy_from_slice(&self.x);
        key[33..65].copy_from_slice(&self.y);
        key
    }

    pub fn from_cbor_bytes(encoded: &[u8]) -> Result<Self, ProtocolError> {
        let Value::Map(fields) = CborCodec::decode(encoded)? else {
            return Err(unexpected("keyAgreement", "a map"));
        };
        let mut key_type = None;
        let mut algorithm = None;
        let mut curve = None;
        let mut x = None;
        let mut y = None;
        for (label, value) in fields {
            let slot = match label {
                Value::Unsigned(1) => &mut key_type,
                Value::Unsigned(3) => &mut algorithm,
                Value::Negative(-1) => &mut curve,
                Value::Negative(-2) => {
                    if x.replace(coordinate(value, "keyAgreement x")?).is_some() {
                        return Err(ProtocolError::DuplicateValue {
                            field: "keyAgreement x",
                        });
                    }
                    continue;
                }
                Value::Negative(-3) => {
                    if y.replace(coordinate(value, "keyAgreement y")?).is_some() {
                        return Err(ProtocolError::DuplicateValue {
                            field: "keyAgreement y",
                        });
                    }
                    continue;
                }
                _ => {
                    return Err(ProtocolError::InvalidValue {
                        field: "keyAgreement",
                        expected: "only the kty, alg, crv, x, and y COSE key parameters",
                    });
                }
            };
            if slot
                .replace(integer(value, "keyAgreement parameter")?)
                .is_some()
            {
                return Err(ProtocolError::DuplicateValue {
                    field: "keyAgreement parameter",
                });
            }
        }
        if key_type != Some(KEY_TYPE_EC2)
            || algorithm != Some(ALGORITHM_ECDH_ES_HKDF_256)
            || curve != Some(CURVE_P256)
        {
            return Err(unexpected(
                "keyAgreement",
                "an EC2 P-256 key with algorithm -25",
            ));
        }
        Ok(Self {
            x: x.ok_or(ProtocolError::MissingField("keyAgreement x"))?,
            y: y.ok_or(ProtocolError::MissingField("keyAgreement y"))?,
        })
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(CborCodec::encode(Value::Map(vec![
            (Value::integer(1), Value::integer(KEY_TYPE_EC2)),
            (
                Value::integer(3),
                Value::integer(ALGORITHM_ECDH_ES_HKDF_256),
            ),
            (Value::integer(-1), Value::integer(CURVE_P256)),
            (Value::integer(-2), Value::ByteString(self.x.to_vec())),
            (Value::integer(-3), Value::ByteString(self.y.to_vec())),
        ]))?)
    }
}

fn coordinate(value: Value, field: &'static str) -> Result<[u8; 32], ProtocolError> {
    let Value::ByteString(value) = value else {
        return Err(unexpected(field, "a 32-byte byte string"));
    };
    value
        .try_into()
        .map_err(|value: Vec<u8>| ProtocolError::InvalidLength {
            field,
            expected: 32,
            actual: value.len(),
        })
}

fn integer(value: Value, field: &'static str) -> Result<i64, ProtocolError> {
    match value {
        Value::Unsigned(value) => i64::try_from(value).map_err(|_| unexpected(field, "an integer")),
        Value::Negative(value) => {
            i64::try_from(value).map_err(|_| unexpected(field, "a signed 64-bit integer"))
        }
        _ => Err(unexpected(field, "an integer")),
    }
}

fn unexpected(field: &'static str, expected: &'static str) -> ProtocolError {
    ProtocolError::UnexpectedType { field, expected }
}
