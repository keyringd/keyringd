use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::CodecError;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct JsonCodec;

impl JsonCodec {
    pub(crate) fn decode<T>(bytes: &[u8]) -> Result<T, CodecError>
    where
        T: DeserializeOwned,
    {
        let mut deserializer = serde_json::Deserializer::from_slice(bytes);
        let value = T::deserialize(&mut deserializer).map_err(CodecError::InvalidJson)?;
        deserializer.end().map_err(CodecError::InvalidJson)?;
        Ok(value)
    }

    pub(crate) fn encode<T>(value: &T) -> Result<Vec<u8>, CodecError>
    where
        T: Serialize,
    {
        serde_json::to_vec(value).map_err(CodecError::EncodeJson)
    }
}
