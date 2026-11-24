use crate::codec::CborItem as Value;

use super::codec::{bytes, decode_canonical_map, encode_map};
use crate::{EncodedCborValue, ProtocolError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridUpdateMessage {
    pub(crate) padding: Option<Vec<u8>>,
    /// Retained canonically but not interpreted because state-assisted
    /// transactions are outside keyringd's initial scope.
    pub(crate) linking_information: Option<EncodedCborValue>,
}

impl HybridUpdateMessage {
    pub fn padding(&self) -> Option<&[u8]> {
        self.padding.as_deref()
    }

    pub const fn linking_information(&self) -> Option<&EncodedCborValue> {
        self.linking_information.as_ref()
    }

    pub fn from_cbor_bytes(encoded: &[u8]) -> Result<Self, ProtocolError> {
        let mut fields = decode_canonical_map(encoded, "hybrid update message")?;
        let padding = fields
            .remove(&0)
            .map(|value| bytes(value, "hybrid update padding"))
            .transpose()?;
        validate_padding(padding.as_deref())?;
        let linking_information = fields
            .remove(&1)
            .map(|value| {
                if !matches!(value, Value::Map(_)) {
                    return Err(ProtocolError::UnexpectedType {
                        field: "hybrid update linking information",
                        expected: "a map",
                    });
                }
                Ok(EncodedCborValue::from_value(value)?)
            })
            .transpose()?;
        Ok(Self {
            padding,
            linking_information,
        })
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        validate_padding(self.padding.as_deref())?;
        let mut fields = Vec::new();
        if let Some(padding) = &self.padding {
            fields.push((0, Value::ByteString(padding.clone())));
        }
        if let Some(linking_information) = &self.linking_information {
            let value = linking_information.to_value()?;
            if !matches!(value, Value::Map(_)) {
                return Err(ProtocolError::UnexpectedType {
                    field: "hybrid update linking information",
                    expected: "a map",
                });
            }
            fields.push((1, value));
        }
        encode_map(fields)
    }
}

fn validate_padding(padding: Option<&[u8]>) -> Result<(), ProtocolError> {
    if padding.is_some_and(|bytes| bytes.iter().any(|byte| *byte != 0)) {
        return Err(ProtocolError::UnexpectedType {
            field: "hybrid update padding",
            expected: "a byte string containing only zero bytes",
        });
    }
    Ok(())
}
