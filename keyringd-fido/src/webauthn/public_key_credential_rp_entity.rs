use crate::codec::CborItem as Value;

use crate::ProtocolError;
use crate::codec::CborValue;

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PublicKeyCredentialRpEntity {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip, default = "default_true")]
    ctap_name_present: bool,
}

const fn default_true() -> bool {
    true
}

impl PublicKeyCredentialRpEntity {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id: None,
            ctap_name_present: true,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Constructs the representation CTAP permits when only an RP ID is present.
    pub fn from_rp_id(id: impl Into<String>) -> Self {
        Self {
            name: String::new(),
            id: Some(id.into()),
            ctap_name_present: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub(crate) fn from_cbor_value(value: Value) -> Result<Self, ProtocolError> {
        let mut fields = CborValue::text_map(value, "rp")?;
        let id = CborValue::text(
            fields
                .remove("id")
                .ok_or(ProtocolError::MissingField("rp.id"))?,
            "rp.id",
        )?;
        let name_value = fields.remove("name");
        let ctap_name_present = name_value.is_some();
        let name = name_value
            .map(|value| CborValue::text(value, "rp.name"))
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            name,
            id: Some(id),
            ctap_name_present,
        })
    }

    pub(crate) fn to_cbor_value(&self) -> Result<Value, ProtocolError> {
        let id = self
            .id
            .as_ref()
            .ok_or(ProtocolError::MissingField("rp.id"))?;
        let mut fields = vec![(
            Value::TextString("id".to_owned()),
            Value::TextString(id.clone()),
        )];
        if self.ctap_name_present {
            fields.push((
                Value::TextString("name".to_owned()),
                Value::TextString(self.name.clone()),
            ));
        }
        Ok(Value::Map(fields))
    }
}
