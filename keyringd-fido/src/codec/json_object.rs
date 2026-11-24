use std::ops::{Deref, DerefMut};

use serde::Serialize;
use serde_json::{Map, Value};

use crate::ProtocolError;

use super::{JsonCodec, JsonValue};

pub(crate) struct JsonObject(Map<String, Value>);

impl JsonObject {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        Self::from_value(JsonCodec::decode(bytes)?)
    }

    pub(crate) fn from_value(value: Value) -> Result<Self, ProtocolError> {
        match value {
            Value::Object(value) => Ok(Self(value)),
            _ => Err(ProtocolError::UnexpectedType {
                field: "JSON value",
                expected: "an object",
            }),
        }
    }

    pub(crate) fn take_value(&mut self, name: &'static str) -> Result<Value, ProtocolError> {
        self.remove(name).ok_or(ProtocolError::MissingField(name))
    }

    pub(crate) fn take<T: serde::de::DeserializeOwned>(
        &mut self,
        name: &'static str,
    ) -> Result<T, ProtocolError> {
        JsonValue::decode(self.take_value(name)?)
    }

    pub(crate) fn take_optional<T: serde::de::DeserializeOwned>(
        &mut self,
        name: &'static str,
    ) -> Result<Option<T>, ProtocolError> {
        self.remove(name).map(JsonValue::decode).transpose()
    }

    pub(crate) fn take_or_default<T: serde::de::DeserializeOwned + Default>(
        &mut self,
        name: &'static str,
    ) -> Result<T, ProtocolError> {
        Ok(self.take_optional(name)?.unwrap_or_default())
    }

    pub(crate) fn into_value(self) -> Value {
        Value::Object(self.0)
    }

    pub(crate) fn into_bytes(self) -> Result<Vec<u8>, ProtocolError> {
        Ok(JsonCodec::encode(&self.into_value())?)
    }

    pub(crate) fn take_binary(&mut self, name: &'static str) -> Result<Vec<u8>, ProtocolError> {
        JsonValue::decode_binary(self.take(name)?)
    }

    pub(crate) fn take_optional_binary(
        &mut self,
        name: &'static str,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        self.take_optional::<String>(name)?
            .map(JsonValue::decode_binary)
            .transpose()
    }

    pub(crate) fn put<T: Serialize + ?Sized>(
        &mut self,
        name: &'static str,
        value: &T,
    ) -> Result<(), ProtocolError> {
        self.insert(name.into(), JsonValue::encode(value)?);
        Ok(())
    }

    pub(crate) fn put_optional<T: Serialize + ?Sized>(
        &mut self,
        name: &'static str,
        value: Option<&T>,
    ) -> Result<(), ProtocolError> {
        if let Some(value) = value {
            self.put(name, value)?;
        }
        Ok(())
    }

    pub(crate) fn put_binary(&mut self, name: &'static str, value: &[u8]) {
        self.insert(name.into(), Value::String(JsonValue::encode_binary(value)));
    }

    pub(crate) fn put_optional_binary(&mut self, name: &'static str, value: Option<&[u8]>) {
        if let Some(value) = value {
            self.put_binary(name, value);
        }
    }
}

impl Default for JsonObject {
    fn default() -> Self {
        Self(Map::new())
    }
}

impl Deref for JsonObject {
    type Target = Map<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for JsonObject {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
