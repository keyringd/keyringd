use crate::codec::CborItem as Value;

use super::{AuthenticatorTransport, PublicKeyCredentialType};
use crate::ProtocolError;
use crate::codec::{CborValue, JsonObject, JsonValue};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicKeyCredentialDescriptor {
    credential_type: PublicKeyCredentialType,
    id: Vec<u8>,
    transports: Option<Vec<AuthenticatorTransport>>,
}

impl PublicKeyCredentialDescriptor {
    pub(crate) fn from_json_value(value: serde_json::Value) -> Result<Self, ProtocolError> {
        let mut fields = JsonObject::from_value(value)?;
        let descriptor = Self::new(fields.take("type")?, fields.take_binary("id")?);
        Ok(match fields.take_optional("transports")? {
            Some(value) => descriptor.with_transports(value),
            None => descriptor,
        })
    }

    pub(crate) fn vec_from_json_value(
        value: Option<serde_json::Value>,
    ) -> Result<Vec<Self>, ProtocolError> {
        let Some(value) = value else {
            return Ok(Vec::new());
        };
        let values: Vec<serde_json::Value> = JsonValue::decode(value)?;
        values.into_iter().map(Self::from_json_value).collect()
    }

    pub(crate) fn to_json_value(&self) -> Result<serde_json::Value, ProtocolError> {
        let mut fields = JsonObject::default();
        fields.put("type", &self.credential_type())?;
        fields.put_binary("id", self.id());
        fields.put_optional("transports", self.transports())?;
        Ok(fields.into_value())
    }

    pub fn new(credential_type: PublicKeyCredentialType, id: Vec<u8>) -> Self {
        Self {
            credential_type,
            id,
            transports: None,
        }
    }

    pub fn with_transports(mut self, transports: Vec<AuthenticatorTransport>) -> Self {
        self.transports = Some(transports);
        self
    }

    pub fn credential_type(&self) -> &PublicKeyCredentialType {
        &self.credential_type
    }
    pub fn id(&self) -> &[u8] {
        &self.id
    }
    pub fn transports(&self) -> Option<&[AuthenticatorTransport]> {
        self.transports.as_deref()
    }

    pub(crate) fn from_cbor_value(value: Value) -> Result<Self, ProtocolError> {
        let mut fields = CborValue::text_map(value, "credential descriptor")?;
        let credential_type = CborValue::text(
            fields
                .remove("type")
                .ok_or(ProtocolError::MissingField("credential.type"))?,
            "credential.type",
        )?;
        if credential_type != "public-key" {
            return Err(CborValue::unexpected("credential.type", "public-key"));
        }
        let id = CborValue::bytes(
            fields
                .remove("id")
                .ok_or(ProtocolError::MissingField("credential.id"))?,
            "credential.id",
        )?;
        let transports = fields
            .remove("transports")
            .map(
                |value| -> Result<Vec<AuthenticatorTransport>, ProtocolError> {
                    Ok(
                        CborValue::text_array(value, "credential.transports", false)?
                            .into_iter()
                            .map(AuthenticatorTransport::from)
                            .collect(),
                    )
                },
            )
            .transpose()?;
        Ok(Self {
            credential_type: PublicKeyCredentialType::PublicKey,
            id,
            transports,
        })
    }

    pub(crate) fn to_cbor_value(&self) -> Result<Value, ProtocolError> {
        let mut fields = vec![
            (
                Value::TextString("type".to_owned()),
                Value::TextString(self.credential_type.as_str().to_owned()),
            ),
            (
                Value::TextString("id".to_owned()),
                Value::ByteString(self.id.clone()),
            ),
        ];
        if let Some(transports) = &self.transports {
            fields.push((
                Value::TextString("transports".to_owned()),
                Value::Array(
                    transports
                        .iter()
                        .map(|transport| Value::TextString(transport.as_str().to_owned()))
                        .collect(),
                ),
            ));
        }
        Ok(Value::Map(fields))
    }

    pub(crate) fn vec_from_cbor_value(
        value: Value,
        field: &'static str,
    ) -> Result<Vec<Self>, ProtocolError> {
        let Value::Array(values) = value else {
            return Err(CborValue::unexpected(field, "an array"));
        };
        if values.is_empty() {
            return Err(ProtocolError::EmptyValue { field });
        }
        values.into_iter().map(Self::from_cbor_value).collect()
    }

    pub(crate) fn vec_to_cbor_value(values: &[Self]) -> Result<Value, ProtocolError> {
        values
            .iter()
            .map(Self::to_cbor_value)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array)
    }
}
