#[derive(Clone, Debug)]
pub enum CborItem {
    Unsigned(u64),
    Negative(i128),
    ByteString(Vec<u8>),
    TextString(String),
    Array(Vec<Self>),
    Map(Vec<(Self, Self)>),
    #[allow(dead_code)]
    // Constructed by encoder-side rejection tests; wire decoding rejects tags eagerly.
    Tag(u64, Box<Self>),
    Simple(CborSimpleValue),
    Float16(u16),
    Float32(u32),
    Float64(u64),
}

use super::CborSimpleValue;

impl CborItem {
    pub fn from_cbor_bytes(bytes: &[u8]) -> Result<Self, crate::CodecError> {
        super::CborCodec::decode(bytes)
    }

    pub fn into_cbor_bytes(self) -> Result<Vec<u8>, crate::CodecError> {
        super::CborCodec::encode(self)
    }

    pub(crate) fn integer(value: i64) -> Self {
        if value >= 0 {
            Self::Unsigned(value as u64)
        } else {
            Self::Negative(i128::from(value))
        }
    }

    pub(crate) fn bool_value(value: bool) -> Self {
        Self::Simple(if value {
            CborSimpleValue::TrueValue
        } else {
            CborSimpleValue::FalseValue
        })
    }
}

impl From<u64> for CborItem {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}

impl From<i64> for CborItem {
    fn from(value: i64) -> Self {
        Self::integer(value)
    }
}

impl From<bool> for CborItem {
    fn from(value: bool) -> Self {
        Self::bool_value(value)
    }
}

impl PartialEq for CborItem {
    fn eq(&self, other: &Self) -> bool {
        use CborItem::{
            Array, ByteString, Float16, Float32, Float64, Map, Negative, Simple, Tag, TextString,
            Unsigned,
        };
        match (self, other) {
            (Unsigned(left), Unsigned(right)) => left == right,
            (Negative(left), Negative(right)) => left == right,
            (ByteString(left), ByteString(right)) => left == right,
            (TextString(left), TextString(right)) => left == right,
            (Array(left), Array(right)) => left == right,
            (Map(left), Map(right)) => left == right,
            (Tag(left_tag, left), Tag(right_tag, right)) => left_tag == right_tag && left == right,
            (Simple(left), Simple(right)) => left == right,
            (Float16(left), Float16(right)) => left == right,
            (Float32(left), Float32(right)) => left == right,
            (Float64(left), Float64(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for CborItem {}
