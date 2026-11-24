use base64::Engine;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{CodecError, ProtocolError};

pub(crate) struct JsonValue;

impl JsonValue {
    pub(crate) fn decode<T: DeserializeOwned>(value: Value) -> Result<T, ProtocolError> {
        serde_json::from_value(value)
            .map_err(|error| ProtocolError::Codec(CodecError::InvalidJson(error)))
    }

    pub(crate) fn encode<T: Serialize + ?Sized>(value: &T) -> Result<Value, ProtocolError> {
        serde_json::to_value(value)
            .map_err(|error| ProtocolError::Codec(CodecError::EncodeJson(error)))
    }

    pub(crate) fn encode_binary(value: &[u8]) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(value)
    }

    pub(crate) fn decode_binary(value: String) -> Result<Vec<u8>, ProtocolError> {
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&value)
            .map_err(CodecError::InvalidBase64Url)?;
        if Self::encode_binary(&decoded) != value {
            return Err(CodecError::NonCanonicalBase64Url.into());
        }
        Ok(decoded)
    }
}
