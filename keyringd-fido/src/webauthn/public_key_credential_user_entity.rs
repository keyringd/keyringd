use crate::codec::CborItem as Value;

use crate::ProtocolError;
use crate::codec::CborValue;
use crate::codec::JsonObject;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicKeyCredentialUserEntity {
    name: Option<String>,
    id: Vec<u8>,
    display_name: Option<String>,
}

impl PublicKeyCredentialUserEntity {
    pub(crate) fn from_json_value(value: serde_json::Value) -> Result<Self, ProtocolError> {
        let mut fields = JsonObject::from_value(value)?;
        let user = Self::new(
            fields.take::<String>("name")?,
            fields.take_binary("id")?,
            fields.take::<String>("displayName")?,
        );
        user.validate_webauthn()?;
        Ok(user)
    }

    pub(crate) fn to_json_value(&self) -> Result<serde_json::Value, ProtocolError> {
        self.validate_webauthn()?;
        let mut fields = JsonObject::default();
        fields.put_binary("id", self.id());
        fields.put(
            "name",
            self.name()
                .ok_or(ProtocolError::MissingField("user.name"))?,
        )?;
        fields.put(
            "displayName",
            self.display_name()
                .ok_or(ProtocolError::MissingField("user.displayName"))?,
        )?;
        Ok(fields.into_value())
    }

    pub fn new(name: impl Into<String>, id: Vec<u8>, display_name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            id,
            display_name: Some(display_name.into()),
        }
    }

    /// Constructs the privacy-preserving representation CTAP permits with only a user ID.
    pub fn from_id(id: Vec<u8>) -> Self {
        Self {
            name: None,
            id,
            display_name: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    pub fn id(&self) -> &[u8] {
        &self.id
    }
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    fn validate(&self) -> Result<(), ProtocolError> {
        if self.id.len() > 64 {
            return Err(ProtocolError::InvalidValue {
                field: "user.id",
                expected: "no more than 64 bytes",
            });
        }
        Ok(())
    }

    pub(crate) fn validate_webauthn(&self) -> Result<(), ProtocolError> {
        self.validate()?;
        if self.id.is_empty() {
            return Err(ProtocolError::EmptyValue { field: "user.id" });
        }
        self.name
            .as_ref()
            .ok_or(ProtocolError::MissingField("user.name"))?;
        self.display_name
            .as_ref()
            .ok_or(ProtocolError::MissingField("user.displayName"))?;
        Ok(())
    }

    pub(crate) fn from_cbor_value(value: Value) -> Result<Self, ProtocolError> {
        let mut fields = CborValue::text_map(value, "user")?;
        let id = CborValue::bytes(
            fields
                .remove("id")
                .ok_or(ProtocolError::MissingField("user.id"))?,
            "user.id",
        )?;
        let name = fields
            .remove("name")
            .map(|value| CborValue::text(value, "user.name"))
            .transpose()?;
        let display_name = fields
            .remove("displayName")
            .map(|value| CborValue::text(value, "user.displayName"))
            .transpose()?;
        let user = Self {
            name,
            id,
            display_name,
        };
        user.validate()?;
        Ok(user)
    }

    pub(crate) fn to_cbor_value(&self) -> Result<Value, ProtocolError> {
        self.validate()?;
        let mut fields = vec![(
            Value::TextString("id".to_owned()),
            Value::ByteString(self.id.clone()),
        )];
        if let Some(name) = &self.name {
            fields.push((
                Value::TextString("name".to_owned()),
                Value::TextString(name.clone()),
            ));
        }
        if let Some(display_name) = &self.display_name {
            fields.push((
                Value::TextString("displayName".to_owned()),
                Value::TextString(display_name.clone()),
            ));
        }
        Ok(Value::Map(fields))
    }
}
