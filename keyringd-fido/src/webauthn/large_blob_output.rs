use crate::ProtocolError;

/// Client extension output; absence of Read data means no blob was obtained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LargeBlobOutput {
    Supported(bool),
    Read(Option<Vec<u8>>),
    Written(bool),
}
impl LargeBlobOutput {
    pub(crate) fn from_json_value(value: serde_json::Value) -> Result<Self, ProtocolError> {
        let mut fields = crate::codec::JsonObject::from_value(value)?;
        let supported = fields.take_optional::<bool>("supported")?;
        let written = fields.take_optional::<bool>("written")?;
        let blob = fields.take_optional_binary("blob")?;
        match (supported, written, blob) {
            (Some(v), None, None) => Ok(Self::Supported(v)),
            (None, Some(v), None) => Ok(Self::Written(v)),
            (None, None, blob) => Ok(Self::Read(blob)),
            _ => Err(ProtocolError::UnexpectedType {
                field: "largeBlob",
                expected: "one operation's output",
            }),
        }
    }
    pub(crate) fn to_json_value(&self) -> Result<serde_json::Value, ProtocolError> {
        Ok(match self {
            Self::Supported(v) => serde_json::json!({"supported":v}),
            Self::Written(v) => serde_json::json!({"written":v}),
            Self::Read(blob) => {
                let mut fields = crate::codec::JsonObject::default();
                fields.put_optional_binary("blob", blob.as_deref());
                fields.into_value()
            }
        })
    }
}
