use crate::codec::CborItem as Value;

use super::{FidoU2fAttestationStatement, PackedAttestationStatement};
use crate::{EncodedCborValue, ProtocolError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttestationStatement {
    None,
    Packed(PackedAttestationStatement),
    FidoU2f(FidoU2fAttestationStatement),
    Unknown {
        format: String,
        statement: EncodedCborValue,
    },
}

impl AttestationStatement {
    pub fn from_cbor_bytes(format: &str, encoded: &[u8]) -> Result<Self, ProtocolError> {
        Self::from_cbor_value(format, crate::codec::CborCodec::decode(encoded)?)
    }

    pub(crate) fn from_cbor_value(format: &str, value: Value) -> Result<Self, ProtocolError> {
        if !matches!(value, Value::Map(_)) {
            return Err(ProtocolError::UnexpectedType {
                field: "attStmt",
                expected: "a map",
            });
        }
        match format {
            "none" => match value {
                Value::Map(fields) if fields.is_empty() => Ok(Self::None),
                _ => Err(ProtocolError::UnexpectedType {
                    field: "none attStmt",
                    expected: "an empty map",
                }),
            },
            "packed" => PackedAttestationStatement::from_cbor_value(value).map(Self::Packed),
            "fido-u2f" => FidoU2fAttestationStatement::from_cbor_value(value).map(Self::FidoU2f),
            _ => Ok(Self::Unknown {
                format: format.to_owned(),
                statement: EncodedCborValue::from_value(value)?,
            }),
        }
    }

    pub fn format(&self) -> &str {
        match self {
            Self::None => "none",
            Self::Packed(_) => "packed",
            Self::FidoU2f(_) => "fido-u2f",
            Self::Unknown { format, .. } => format,
        }
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(crate::codec::CborCodec::encode(self.to_cbor_value()?)?)
    }

    pub(crate) fn to_cbor_value(&self) -> Result<Value, ProtocolError> {
        Ok(match self {
            Self::None => Value::Map(Vec::new()),
            Self::Packed(statement) => statement.to_cbor_value()?,
            Self::FidoU2f(statement) => statement.to_cbor_value()?,
            Self::Unknown { statement, .. } => statement.to_value()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "AttestationStatement::from_cbor_bytes" {
            case "Keep an unknown Attestation format" {
                why "Attestation format registries are extensible, so the codec must preserve an unknown format and statement instead of corrupting data it cannot interpret."
                spec "https://www.w3.org/TR/webauthn-3/#sctn-attstn-fmt-ids"
                verify "Keep an unknown Attestation format" {
                    let value =
                        AttestationStatement::from_cbor_bytes("future-format", &[0xa1, 0x01, 0x02]).unwrap();
                    assert_eq!(value.format(), "future-format");
                    assert_eq!(value.to_cbor_bytes().unwrap(), vec![0xa1, 0x01, 0x02]);
                }
            }
        }
    }
}
