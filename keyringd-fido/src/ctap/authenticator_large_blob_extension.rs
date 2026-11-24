use crate::{
    EncodedCborValue, ProtocolError,
    codec::{CborCodec, CborItem as Value},
};

/// The CTAP `largeBlob` extension carries compressed blob data.
pub enum AuthenticatorLargeBlobExtension {
    Support {
        required: bool,
    },
    Read,
    Write {
        compressed: Vec<u8>,
        original_size: u64,
    },
}
impl AuthenticatorLargeBlobExtension {
    pub fn to_encoded_value(&self) -> Result<EncodedCborValue, ProtocolError> {
        let fields = match self {
            Self::Support { required } => vec![(
                Value::TextString("support".into()),
                Value::TextString(if *required { "required" } else { "preferred" }.into()),
            )],
            Self::Read => vec![(Value::TextString("read".into()), Value::bool_value(true))],
            Self::Write {
                compressed,
                original_size,
            } => vec![
                (
                    Value::TextString("write".into()),
                    Value::ByteString(compressed.clone()),
                ),
                (
                    Value::TextString("originalSize".into()),
                    (*original_size).into(),
                ),
            ],
        };
        Ok(EncodedCborValue::from_canonical_bytes(CborCodec::encode(
            Value::Map(fields),
        )?)?)
    }
}
