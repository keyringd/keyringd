use std::collections::BTreeMap;

use crate::codec::CborItem as Value;

use super::CoseAlgorithmIdentifier;
use crate::ProtocolError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackedAttestationStatement {
    pub(crate) algorithm: CoseAlgorithmIdentifier,
    pub(crate) signature: Vec<u8>,
    pub(crate) certificate_chain: Option<Vec<Vec<u8>>>,
}

impl PackedAttestationStatement {
    pub fn new(algorithm: CoseAlgorithmIdentifier, signature: Vec<u8>) -> Self {
        Self {
            algorithm,
            signature,
            certificate_chain: None,
        }
    }

    pub fn with_certificate_chain(mut self, value: Vec<Vec<u8>>) -> Self {
        self.certificate_chain = Some(value);
        self
    }

    pub const fn algorithm(&self) -> CoseAlgorithmIdentifier {
        self.algorithm
    }

    pub fn signature(&self) -> &[u8] {
        &self.signature
    }

    pub fn certificate_chain(&self) -> Option<&[Vec<u8>]> {
        self.certificate_chain.as_deref()
    }

    pub(crate) fn from_cbor_value(value: Value) -> Result<Self, ProtocolError> {
        let mut fields = text_map(value, "packed attestation statement")?;
        let algorithm = integer(
            fields.remove("alg").ok_or(ProtocolError::UnexpectedType {
                field: "packed.alg",
                expected: "a required integer",
            })?,
            "packed.alg",
        )?;
        let signature = byte_string(
            fields.remove("sig").ok_or(ProtocolError::UnexpectedType {
                field: "packed.sig",
                expected: "a required byte string",
            })?,
            "packed.sig",
        )?;
        let certificate_chain = fields
            .remove("x5c")
            .map(|value| byte_string_array(value, "packed.x5c"))
            .transpose()?;
        Ok(Self {
            algorithm: CoseAlgorithmIdentifier::new(i32::try_from(algorithm).map_err(|_| {
                ProtocolError::UnexpectedType {
                    field: "packed.alg",
                    expected: "a WebIDL long",
                }
            })?),
            signature,
            certificate_chain,
        })
    }

    pub(crate) fn to_cbor_value(&self) -> Result<Value, ProtocolError> {
        let mut fields = vec![
            (
                Value::TextString("alg".to_owned()),
                Value::integer(i64::from(self.algorithm.value())),
            ),
            (
                Value::TextString("sig".to_owned()),
                Value::ByteString(self.signature.clone()),
            ),
        ];
        if let Some(chain) = &self.certificate_chain {
            if chain.is_empty() {
                return Err(ProtocolError::EmptyValue {
                    field: "packed.x5c",
                });
            }
            fields.push((
                Value::TextString("x5c".to_owned()),
                Value::Array(chain.iter().cloned().map(Value::ByteString).collect()),
            ));
        }
        Ok(Value::Map(fields))
    }
}

pub(crate) fn text_map(
    value: Value,
    field: &'static str,
) -> Result<BTreeMap<String, Value>, ProtocolError> {
    let Value::Map(entries) = value else {
        return Err(ProtocolError::UnexpectedType {
            field,
            expected: "a map",
        });
    };
    let mut fields = BTreeMap::new();
    for (key, value) in entries {
        let Value::TextString(key) = key else {
            return Err(ProtocolError::UnexpectedType {
                field,
                expected: "text-string map keys",
            });
        };
        if fields.insert(key, value).is_some() {
            return Err(ProtocolError::DuplicateValue { field });
        }
    }
    Ok(fields)
}

pub(crate) fn byte_string(value: Value, field: &'static str) -> Result<Vec<u8>, ProtocolError> {
    match value {
        Value::ByteString(value) => Ok(value),
        _ => Err(ProtocolError::UnexpectedType {
            field,
            expected: "a byte string",
        }),
    }
}

pub(crate) fn byte_string_array(
    value: Value,
    field: &'static str,
) -> Result<Vec<Vec<u8>>, ProtocolError> {
    let Value::Array(values) = value else {
        return Err(ProtocolError::UnexpectedType {
            field,
            expected: "an array of byte strings",
        });
    };
    if values.is_empty() {
        return Err(ProtocolError::EmptyValue { field });
    }
    values
        .into_iter()
        .map(|value| byte_string(value, field))
        .collect()
}

fn integer(value: Value, field: &'static str) -> Result<i64, ProtocolError> {
    match value {
        Value::Unsigned(value) => i64::try_from(value).map_err(|_| ProtocolError::UnexpectedType {
            field,
            expected: "an integer representable as i64",
        }),
        Value::Negative(value) => i64::try_from(value).map_err(|_| ProtocolError::UnexpectedType {
            field,
            expected: "a signed 64-bit integer",
        }),
        _ => Err(ProtocolError::UnexpectedType {
            field,
            expected: "an integer",
        }),
    }
}
