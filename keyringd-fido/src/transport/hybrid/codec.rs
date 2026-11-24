use std::collections::BTreeMap;

use crate::codec::{CborItem as Value, CborSimpleValue};

use crate::ProtocolError;

pub(super) fn decode_canonical_map(
    encoded: &[u8],
    field: &'static str,
) -> Result<BTreeMap<u64, Value>, ProtocolError> {
    let value = crate::codec::CborCodec::decode(encoded)?;
    if crate::codec::CborCodec::encode(value.clone())? != encoded {
        return Err(ProtocolError::Codec(crate::CodecError::InvalidCbor(
            "non-canonical CBOR encoding".into(),
        )));
    }
    let Value::Map(entries) = value else {
        return Err(ProtocolError::UnexpectedType {
            field,
            expected: "a canonical CBOR map",
        });
    };
    let mut fields = BTreeMap::new();
    for (key, value) in entries {
        let Value::Unsigned(key) = key else {
            return Err(ProtocolError::UnexpectedType {
                field,
                expected: "unsigned integer map keys",
            });
        };
        if fields.insert(key, value).is_some() {
            return Err(ProtocolError::DuplicateValue { field });
        }
    }
    Ok(fields)
}

pub(crate) fn encode_map(fields: Vec<(u64, Value)>) -> Result<Vec<u8>, ProtocolError> {
    let values = fields
        .into_iter()
        .map(|(key, value)| (Value::Unsigned(key), value))
        .collect::<Vec<_>>();
    Ok(crate::codec::CborCodec::encode(Value::Map(values))?)
}

pub(super) fn bytes(value: Value, field: &'static str) -> Result<Vec<u8>, ProtocolError> {
    match value {
        Value::ByteString(bytes) => Ok(bytes),
        _ => Err(ProtocolError::UnexpectedType {
            field,
            expected: "a byte string",
        }),
    }
}

pub(super) fn text(value: Value, field: &'static str) -> Result<String, ProtocolError> {
    match value {
        Value::TextString(text) => Ok(text),
        _ => Err(ProtocolError::UnexpectedType {
            field,
            expected: "a text string",
        }),
    }
}

pub(super) fn unsigned(value: Value, field: &'static str) -> Result<u64, ProtocolError> {
    match value {
        Value::Unsigned(value) => Ok(value),
        _ => Err(ProtocolError::UnexpectedType {
            field,
            expected: "an unsigned integer",
        }),
    }
}

pub(super) fn boolean(value: Value, field: &'static str) -> Result<bool, ProtocolError> {
    match value {
        Value::Simple(CborSimpleValue::TrueValue) => Ok(true),
        Value::Simple(CborSimpleValue::FalseValue) => Ok(false),
        _ => Err(ProtocolError::UnexpectedType {
            field,
            expected: "a boolean",
        }),
    }
}
