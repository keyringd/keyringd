use crate::CodecError;
use crate::codec::{CborItem as Value, CborSimpleValue};

const MAX_NESTING_DEPTH: u8 = 4;
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CborCodec;

impl CborCodec {
    pub(crate) fn decode(bytes: &[u8]) -> Result<Value, CodecError> {
        let (value, length) = decode_item(bytes, 0)?;
        if length != bytes.len() {
            return Err(invalid("trailing CBOR data"));
        }
        Ok(value)
    }

    pub(crate) fn encode(value: Value) -> Result<Vec<u8>, CodecError> {
        let mut bytes = Vec::new();
        encode_item(&value, &mut bytes, 0)?;
        Ok(bytes)
    }

    pub(crate) fn encoded_item_length(bytes: &[u8]) -> Result<usize, CodecError> {
        decode_item(bytes, 0).map(|(_, length)| length)
    }
}

fn invalid(message: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> CodecError {
    CodecError::InvalidCbor(message.into())
}

fn decode_item(bytes: &[u8], container_depth: u8) -> Result<(Value, usize), CodecError> {
    let initial = *bytes
        .first()
        .ok_or_else(|| invalid("incomplete CBOR item"))?;
    let major = initial >> 5;
    let additional = initial & 0x1f;
    if major == 7 {
        return decode_simple(bytes, additional);
    }
    let (argument, header_length) = decode_argument(bytes, additional)?;
    match major {
        0 => Ok((Value::Unsigned(argument), header_length)),
        1 => Ok((Value::Negative(-1 - i128::from(argument)), header_length)),
        2 | 3 => {
            let length = usize::try_from(argument)
                .map_err(|_| invalid("CBOR string length does not fit usize"))?;
            let end = header_length
                .checked_add(length)
                .filter(|end| *end <= bytes.len())
                .ok_or_else(|| invalid("truncated CBOR string"))?;
            let contents = &bytes[header_length..end];
            let value = if major == 2 {
                Value::ByteString(contents.to_vec())
            } else {
                Value::TextString(std::str::from_utf8(contents).map_err(invalid)?.to_owned())
            };
            Ok((value, end))
        }
        4 => {
            let depth = next_container_depth(container_depth)?;
            let count = usize::try_from(argument)
                .map_err(|_| invalid("CBOR array length does not fit usize"))?;
            if count > bytes.len().saturating_sub(header_length) {
                return Err(invalid(
                    "declared CBOR array length exceeds the remaining input",
                ));
            }
            let mut values = Vec::with_capacity(count);
            let mut offset = header_length;
            for _ in 0..count {
                let (value, length) = decode_item(remaining(bytes, offset)?, depth)?;
                offset = checked_add(offset, length)?;
                values.push(value);
            }
            Ok((Value::Array(values), offset))
        }
        5 => {
            let depth = next_container_depth(container_depth)?;
            let count = usize::try_from(argument)
                .map_err(|_| invalid("CBOR map length does not fit usize"))?;
            if count > bytes.len().saturating_sub(header_length) / 2 {
                return Err(invalid(
                    "declared CBOR map length exceeds the remaining input",
                ));
            }
            let mut entries = Vec::with_capacity(count);
            let mut offset = header_length;
            let mut previous_key: Option<&[u8]> = None;
            for _ in 0..count {
                let key_start = offset;
                let (key, key_length) = decode_item(remaining(bytes, offset)?, depth)?;
                offset = checked_add(offset, key_length)?;
                if matches!(key, Value::Array(_) | Value::Map(_) | Value::Tag(_, _)) {
                    return Err(invalid(
                        "complex CBOR values are not permitted as CTAP map keys",
                    ));
                }
                let encoded_key = &bytes[key_start..offset];
                if let Some(previous) = previous_key {
                    match compare_encoded_keys(previous, encoded_key) {
                        std::cmp::Ordering::Less => {}
                        std::cmp::Ordering::Equal => {
                            return Err(invalid("duplicate CBOR map key"));
                        }
                        std::cmp::Ordering::Greater => {
                            return Err(invalid("non-canonical CBOR map key order"));
                        }
                    }
                }
                previous_key = Some(encoded_key);
                let (value, value_length) = decode_item(remaining(bytes, offset)?, depth)?;
                offset = checked_add(offset, value_length)?;
                entries.push((key, value));
            }
            Ok((Value::Map(entries), offset))
        }
        6 => Err(invalid("CBOR tags are not permitted by CTAP")),
        _ => Err(invalid("unsupported CBOR major type")),
    }
}

fn decode_simple(bytes: &[u8], additional: u8) -> Result<(Value, usize), CodecError> {
    let simple = |value| Ok((Value::Simple(value), 1));
    match additional {
        value @ 0..=19 => simple(CborSimpleValue::Other(value)),
        20 => simple(CborSimpleValue::FalseValue),
        21 => simple(CborSimpleValue::TrueValue),
        22 => simple(CborSimpleValue::NullValue),
        23 => simple(CborSimpleValue::Undefined),
        24 => {
            let value = *bytes
                .get(1)
                .ok_or_else(|| invalid("truncated CBOR simple value"))?;
            if value < 32 {
                return Err(invalid("non-canonical CBOR simple value"));
            }
            Ok((Value::Simple(CborSimpleValue::Other(value)), 2))
        }
        25 => Ok((Value::Float16(read_u16(bytes, 1)?), 3)),
        26 => Ok((Value::Float32(read_u32(bytes, 1)?), 5)),
        27 => Ok((Value::Float64(read_u64(bytes, 1)?), 9)),
        _ => Err(invalid(
            "indefinite-length or reserved CBOR additional information",
        )),
    }
}

fn decode_argument(bytes: &[u8], additional: u8) -> Result<(u64, usize), CodecError> {
    let decoded = match additional {
        value @ 0..=23 => (u64::from(value), 1),
        24 => (
            u64::from(
                *bytes
                    .get(1)
                    .ok_or_else(|| invalid("truncated CBOR argument"))?,
            ),
            2,
        ),
        25 => (u64::from(read_u16(bytes, 1)?), 3),
        26 => (u64::from(read_u32(bytes, 1)?), 5),
        27 => (read_u64(bytes, 1)?, 9),
        _ => {
            return Err(invalid(
                "indefinite-length or reserved CBOR additional information",
            ));
        }
    };
    let minimum = match additional {
        24 => 24,
        25 => 1 << 8,
        26 => 1 << 16,
        27 => 1 << 32,
        _ => 0,
    };
    if decoded.0 < minimum {
        return Err(invalid("non-canonical CBOR integer or length"));
    }
    Ok(decoded)
}

fn encode_item(value: &Value, output: &mut Vec<u8>, container_depth: u8) -> Result<(), CodecError> {
    match value {
        Value::Unsigned(value) => encode_argument(0, *value, output),
        Value::Negative(value) if *value < 0 => encode_argument(
            1,
            u64::try_from(-1 - *value)
                .map_err(|_| CodecError::EncodeCbor("CBOR nint is below -2^64".into()))?,
            output,
        ),
        Value::Negative(_) => return Err(CodecError::EncodeCbor("non-negative CBOR nint".into())),
        Value::ByteString(value) => {
            encode_argument(2, value.len() as u64, output);
            output.extend_from_slice(value);
        }
        Value::TextString(value) => {
            encode_argument(3, value.len() as u64, output);
            output.extend_from_slice(value.as_bytes());
        }
        Value::Array(values) => {
            let depth = next_container_depth(container_depth)?;
            encode_argument(4, values.len() as u64, output);
            for value in values {
                encode_item(value, output, depth)?;
            }
        }
        Value::Map(entries) => {
            let depth = next_container_depth(container_depth)?;
            let mut encoded = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                if matches!(key, Value::Array(_) | Value::Map(_) | Value::Tag(_, _)) {
                    return Err(invalid(
                        "complex CBOR values are not permitted as CTAP map keys",
                    ));
                }
                let mut key_bytes = Vec::new();
                encode_item(key, &mut key_bytes, depth)?;
                let mut value_bytes = Vec::new();
                encode_item(value, &mut value_bytes, depth)?;
                encoded.push((key_bytes, value_bytes));
            }
            encoded.sort_by(|left, right| compare_encoded_keys(&left.0, &right.0));
            if encoded.windows(2).any(|pair| pair[0].0 == pair[1].0) {
                return Err(invalid("duplicate CBOR map key"));
            }
            encode_argument(5, encoded.len() as u64, output);
            for (key, value) in encoded {
                output.extend_from_slice(&key);
                output.extend_from_slice(&value);
            }
        }
        Value::Tag(_, _) => return Err(invalid("CBOR tags are not permitted by CTAP")),
        Value::Simple(value) => encode_simple(*value, output)?,
        Value::Float16(bits) => {
            output.push(0xf9);
            output.extend_from_slice(&bits.to_be_bytes());
        }
        Value::Float32(bits) => {
            output.push(0xfa);
            output.extend_from_slice(&bits.to_be_bytes());
        }
        Value::Float64(bits) => {
            output.push(0xfb);
            output.extend_from_slice(&bits.to_be_bytes());
        }
    }
    Ok(())
}

fn encode_simple(value: CborSimpleValue, output: &mut Vec<u8>) -> Result<(), CodecError> {
    let value = match value {
        CborSimpleValue::Other(value @ 20..=31) => {
            return Err(CodecError::EncodeCbor(format!(
                "CBOR simple value {value} is named or reserved and must not use Other"
            )));
        }
        CborSimpleValue::Other(value) => value,
        CborSimpleValue::FalseValue => 20,
        CborSimpleValue::TrueValue => 21,
        CborSimpleValue::NullValue => 22,
        CborSimpleValue::Undefined => 23,
    };
    if value <= 23 {
        output.push(0xe0 | value);
    } else {
        output.extend_from_slice(&[0xf8, value]);
    }
    Ok(())
}

fn encode_argument(major: u8, value: u64, output: &mut Vec<u8>) {
    let prefix = major << 5;
    match value {
        0..=23 => output.push(prefix | value as u8),
        24..=0xff => output.extend_from_slice(&[prefix | 24, value as u8]),
        0x100..=0xffff => {
            output.push(prefix | 25);
            output.extend_from_slice(&(value as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            output.push(prefix | 26);
            output.extend_from_slice(&(value as u32).to_be_bytes());
        }
        _ => {
            output.push(prefix | 27);
            output.extend_from_slice(&value.to_be_bytes());
        }
    }
}

fn next_container_depth(depth: u8) -> Result<u8, CodecError> {
    let depth = depth + 1;
    if depth > MAX_NESTING_DEPTH {
        Err(invalid(
            "CBOR nesting exceeds the CTAP limit of four levels",
        ))
    } else {
        Ok(depth)
    }
}

fn remaining(bytes: &[u8], offset: usize) -> Result<&[u8], CodecError> {
    bytes
        .get(offset..)
        .ok_or_else(|| invalid("truncated CBOR container"))
}

fn checked_add(offset: usize, length: usize) -> Result<usize, CodecError> {
    offset
        .checked_add(length)
        .ok_or_else(|| invalid("CBOR item is too large"))
}

fn compare_encoded_keys(left: &[u8], right: &[u8]) -> std::cmp::Ordering {
    (left[0] >> 5)
        .cmp(&(right[0] >> 5))
        .then(left.len().cmp(&right.len()))
        .then(left.cmp(right))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, CodecError> {
    bytes
        .get(offset..offset + 2)
        .and_then(|value| value.try_into().ok())
        .map(u16::from_be_bytes)
        .ok_or_else(|| invalid("truncated CBOR argument"))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, CodecError> {
    bytes
        .get(offset..offset + 4)
        .and_then(|value| value.try_into().ok())
        .map(u32::from_be_bytes)
        .ok_or_else(|| invalid("truncated CBOR argument"))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, CodecError> {
    bytes
        .get(offset..offset + 8)
        .and_then(|value| value.try_into().ok())
        .map(u64::from_be_bytes)
        .ok_or_else(|| invalid("truncated CBOR argument"))
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "CborCodec::encoded_item_length" {
            case "You can get the first item from a concatenated CBOR" {
                why "The prefix-decoder API must report the exact bytes consumed so callers can safely parse framed or nested data without accepting trailing bytes as part of one value."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#message-encoding"
                verify "You can get the first item from a concatenated CBOR" {
                    assert_eq!(
                        CborCodec::encoded_item_length(&[0xa1, 0x01, 0x82, 0x02, 0x03, 0xf5]).unwrap(),
                        5
                    );
                }
            }
        }
    }
}
