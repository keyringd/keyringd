use super::ProtocolError;
use crate::codec::JsonObject;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorAssertionResponse {
    client_data_json: Vec<u8>,
    authenticator_data: Vec<u8>,
    signature: Vec<u8>,
    user_handle: Option<Vec<u8>>,
}

impl AuthenticatorAssertionResponse {
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        Self::from_json_value(JsonObject::from_bytes(bytes)?.into_value())
    }

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        JsonObject::from_value(self.to_json_value())?.into_bytes()
    }

    pub(crate) fn from_json_value(value: serde_json::Value) -> Result<Self, ProtocolError> {
        let mut fields = JsonObject::from_value(value)?;
        Ok(Self::new(
            fields.take_binary("clientDataJSON")?,
            fields.take_binary("authenticatorData")?,
            fields.take_binary("signature")?,
            fields.take_optional_binary("userHandle")?,
        ))
    }

    pub(crate) fn to_json_value(&self) -> serde_json::Value {
        let mut fields = JsonObject::default();
        fields.put_binary("clientDataJSON", self.client_data_json());
        fields.put_binary("authenticatorData", self.authenticator_data());
        fields.put_binary("signature", self.signature());
        fields.put_optional_binary("userHandle", self.user_handle());
        fields.into_value()
    }

    pub fn new(
        client_data_json: Vec<u8>,
        authenticator_data: Vec<u8>,
        signature: Vec<u8>,
        user_handle: Option<Vec<u8>>,
    ) -> Self {
        Self {
            client_data_json,
            authenticator_data,
            signature,
            user_handle,
        }
    }

    pub fn client_data_json(&self) -> &[u8] {
        &self.client_data_json
    }

    pub fn authenticator_data(&self) -> &[u8] {
        &self.authenticator_data
    }

    pub fn signature(&self) -> &[u8] {
        &self.signature
    }

    pub fn user_handle(&self) -> Option<&[u8]> {
        self.user_handle.as_deref()
    }
}
