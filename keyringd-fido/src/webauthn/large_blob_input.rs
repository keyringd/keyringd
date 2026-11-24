use super::LargeBlobSupport;
use crate::{ProtocolError, codec::JsonObject};

/// WebAuthn largeBlob input dictionary. Member presence is preserved until
/// registration or authentication processing applies the corresponding rules.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LargeBlobInput {
    support: Option<LargeBlobSupport>,
    read: Option<bool>,
    write: Option<Vec<u8>>,
}
impl LargeBlobInput {
    pub fn request_support(support: LargeBlobSupport) -> Self {
        Self {
            support: Some(support),
            ..Self::default()
        }
    }

    pub fn request_read() -> Self {
        Self {
            read: Some(true),
            ..Self::default()
        }
    }

    pub fn request_write(data: Vec<u8>) -> Self {
        Self {
            write: Some(data),
            ..Self::default()
        }
    }

    pub fn support(&self) -> Option<&LargeBlobSupport> {
        self.support.as_ref()
    }
    pub fn read(&self) -> Option<bool> {
        self.read
    }
    pub fn write(&self) -> Option<&[u8]> {
        self.write.as_deref()
    }

    pub fn validate_registration(&self) -> Result<(), ProtocolError> {
        if self.read.is_some() || self.write.is_some() {
            return Err(Self::invalid());
        }
        Ok(())
    }
    pub fn validate_authentication(&self) -> Result<(), ProtocolError> {
        if self.support.is_some() || (self.read.is_some() && self.write.is_some()) {
            return Err(Self::invalid());
        }
        Ok(())
    }
    pub(crate) fn from_json_value(value: serde_json::Value) -> Result<Self, ProtocolError> {
        let mut fields = JsonObject::from_value(value)?;
        Ok(Self {
            support: fields.take_optional("support")?,
            read: fields.take_optional("read")?,
            write: fields.take_optional_binary("write")?,
        })
    }
    pub(crate) fn to_json_value(&self) -> Result<serde_json::Value, ProtocolError> {
        let mut fields = JsonObject::default();
        fields.put_optional("support", self.support.as_ref())?;
        fields.put_optional("read", self.read.as_ref())?;
        fields.put_optional_binary("write", self.write.as_deref());
        Ok(fields.into_value())
    }
    fn invalid() -> ProtocolError {
        ProtocolError::UnexpectedType {
            field: "largeBlob",
            expected: "members valid for the requested ceremony",
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dictionary_preserves_absence_false_unknown_values_and_empty_writes() {
        for json in [
            serde_json::json!({}),
            serde_json::json!({"read":false}),
            serde_json::json!({"support":"required"}),
            serde_json::json!({"support":"preferred"}),
            serde_json::json!({"support":"future-support"}),
            serde_json::json!({"write":""}),
        ] {
            let input = LargeBlobInput::from_json_value(json.clone()).unwrap();
            assert_eq!(input.to_json_value().unwrap(), json);
        }
        let empty = LargeBlobInput::default();
        assert!(empty.validate_registration().is_ok());
        assert!(empty.validate_authentication().is_ok());
        let disabled = LargeBlobInput::from_json_value(serde_json::json!({"read": false})).unwrap();
        assert!(disabled.validate_authentication().is_ok());
        assert!(disabled.validate_registration().is_err());
    }
    #[test]
    fn member_conflicts_are_checked_when_resolving_the_ceremony() {
        let input =
            LargeBlobInput::from_json_value(serde_json::json!({"read": false, "write": ""}))
                .unwrap();
        assert!(input.validate_authentication().is_err());
        let input = LargeBlobInput::from_json_value(
            serde_json::json!({"support": "preferred", "read": true}),
        )
        .unwrap();
        assert!(input.validate_registration().is_err());
        assert!(input.validate_authentication().is_err());
    }

    #[test]
    fn support_values_are_typed_and_only_valid_for_registration() {
        for (wire, support) in [
            ("required", LargeBlobSupport::Required),
            ("preferred", LargeBlobSupport::Preferred),
            (
                "future-support",
                LargeBlobSupport::Unknown("future-support".into()),
            ),
        ] {
            let input =
                LargeBlobInput::from_json_value(serde_json::json!({"support": wire})).unwrap();
            assert_eq!(input.support(), Some(&support));
            assert!(input.validate_registration().is_ok());
            assert!(input.validate_authentication().is_err());
            assert_eq!(LargeBlobInput::request_support(support), input);
        }
    }
}
