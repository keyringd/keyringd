use crate::{
    ProtocolError,
    codec::{CborCodec, CborItem as Value},
};

pub enum AuthenticatorLargeBlobOutput {
    Supported(bool),
    Read {
        compressed: Vec<u8>,
        original_size: u64,
    },
    Written(bool),
    Unavailable,
}
impl AuthenticatorLargeBlobOutput {
    pub fn from_cbor_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let Value::Map(fields) = CborCodec::decode(bytes)? else {
            return Err(Self::invalid());
        };
        let mut supported = None;
        let mut written = None;
        let mut blob = None;
        let mut size = None;
        for (k, v) in fields {
            match (k, v) {
                (Value::TextString(k), Value::Simple(v))
                    if k == "supported" && supported.is_none() =>
                {
                    supported = Some(match v {
                        crate::codec::CborSimpleValue::TrueValue => true,
                        crate::codec::CborSimpleValue::FalseValue => false,
                        _ => return Err(Self::invalid()),
                    })
                }
                (Value::TextString(k), Value::Simple(v)) if k == "written" && written.is_none() => {
                    written = Some(match v {
                        crate::codec::CborSimpleValue::TrueValue => true,
                        crate::codec::CborSimpleValue::FalseValue => false,
                        _ => return Err(Self::invalid()),
                    })
                }
                (Value::TextString(k), Value::ByteString(v)) if k == "blob" && blob.is_none() => {
                    blob = Some(v)
                }
                (Value::TextString(k), Value::Unsigned(v))
                    if k == "originalSize" && size.is_none() =>
                {
                    size = Some(v)
                }
                (Value::TextString(k), _)
                    if matches!(
                        k.as_str(),
                        "supported" | "written" | "blob" | "originalSize"
                    ) =>
                {
                    return Err(Self::invalid());
                }
                _ => {}
            }
        }
        match (supported, written, blob, size) {
            (Some(v), None, None, None) => Ok(Self::Supported(v)),
            (None, Some(v), None, None) => Ok(Self::Written(v)),
            (None, None, Some(compressed), Some(original_size)) => Ok(Self::Read {
                compressed,
                original_size,
            }),
            (None, None, None, None) => Ok(Self::Unavailable),
            _ => Err(Self::invalid()),
        }
    }
    fn invalid() -> ProtocolError {
        ProtocolError::UnexpectedType {
            field: "largeBlob",
            expected: "a valid CTAP largeBlob output",
        }
    }
}
