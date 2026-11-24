use crate::codec::CborItem as Value;

use crate::CodecError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedCborValue(Vec<u8>);

impl EncodedCborValue {
    pub fn from_canonical_bytes(bytes: Vec<u8>) -> Result<Self, CodecError> {
        super::CborCodec::decode(&bytes)?;
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    pub(crate) fn from_value(value: Value) -> Result<Self, CodecError> {
        Ok(Self(super::CborCodec::encode(value)?))
    }

    pub(crate) fn to_value(&self) -> Result<Value, CodecError> {
        super::CborCodec::decode(&self.0)
    }
}
