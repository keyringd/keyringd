use super::CredentialPropertiesOutput;
use crate::{ProtocolError, codec::JsonObject};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuthenticationExtensionsClientOutputs {
    large_blob: Option<super::LargeBlobOutput>,
    credential_properties: Option<CredentialPropertiesOutput>,
}

impl AuthenticationExtensionsClientOutputs {
    fn with_optional_large_blob(mut self, value: Option<super::LargeBlobOutput>) -> Self {
        self.large_blob = value;
        self
    }
    pub fn with_large_blob(self, value: super::LargeBlobOutput) -> Self {
        self.with_optional_large_blob(Some(value))
    }
    pub fn large_blob(&self) -> Option<&super::LargeBlobOutput> {
        self.large_blob.as_ref()
    }

    pub(crate) fn from_json_value(value: serde_json::Value) -> Result<Self, ProtocolError> {
        let mut fields = JsonObject::from_value(value)?;
        let large_blob = fields
            .take_optional::<serde_json::Value>("largeBlob")?
            .map(super::LargeBlobOutput::from_json_value)
            .transpose()?;
        Ok(match fields.take_optional("credProps")? {
            Some(value) => Self::default().with_credential_properties(value),
            None => Self::default(),
        }
        .with_optional_large_blob(large_blob))
    }

    pub(crate) fn to_json_value(&self) -> Result<serde_json::Value, ProtocolError> {
        let mut fields = JsonObject::default();
        if let Some(value) = &self.large_blob {
            fields.put("largeBlob", &value.to_json_value()?)?;
        }
        fields.put_optional("credProps", self.credential_properties())?;
        Ok(fields.into_value())
    }

    pub fn with_credential_properties(mut self, value: CredentialPropertiesOutput) -> Self {
        self.credential_properties = Some(value);
        self
    }

    pub const fn credential_properties(&self) -> Option<&CredentialPropertiesOutput> {
        self.credential_properties.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "AuthenticationExtensionsClientOutputs::from_json_value" {
            case "Don't mix unknown Client Extension outputs into supported models" {
                why "Ignoring unknown JSON members keeps forward-compatible extension output input from appearing as a supported typed result."
                verify "The unknown output member does not exist after conversion to a supported model" {
                    let outputs = AuthenticationExtensionsClientOutputs::from_json_value(
                        serde_json::json!({"ignoredExtension": true}),
                    )
                    .unwrap();
                    assert_eq!(outputs.to_json_value().unwrap(), serde_json::json!({}));
                }
            }
        }
    }
}
