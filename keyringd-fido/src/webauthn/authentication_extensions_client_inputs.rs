use super::CredentialProtectionPolicy;
use crate::{ProtocolError, codec::JsonObject};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuthenticationExtensionsClientInputs {
    large_blob: Option<super::LargeBlobInput>,
    credential_properties: Option<bool>,
    credential_protection_policy: Option<CredentialProtectionPolicy>,
    enforce_credential_protection_policy: Option<bool>,
}

impl AuthenticationExtensionsClientInputs {
    fn with_optional_large_blob(mut self, value: Option<super::LargeBlobInput>) -> Self {
        self.large_blob = value;
        self
    }
    pub fn with_large_blob(self, value: super::LargeBlobInput) -> Self {
        self.with_optional_large_blob(Some(value))
    }
    pub fn large_blob(&self) -> Option<&super::LargeBlobInput> {
        self.large_blob.as_ref()
    }

    pub(crate) fn from_json_value(value: serde_json::Value) -> Result<Self, ProtocolError> {
        let mut fields = JsonObject::from_value(value)?;
        let large_blob = fields
            .take_optional::<serde_json::Value>("largeBlob")?
            .map(super::LargeBlobInput::from_json_value)
            .transpose()?;
        Ok(Self::from_parts(
            fields.take_optional("credProps")?,
            fields
                .take_optional::<String>("credentialProtectionPolicy")?
                .map(CredentialProtectionPolicy::from_string),
            fields.take_optional("enforceCredentialProtectionPolicy")?,
        )
        .with_optional_large_blob(large_blob))
    }

    pub(crate) fn to_json_value(&self) -> Result<serde_json::Value, ProtocolError> {
        let mut fields = JsonObject::default();
        if let Some(value) = &self.large_blob {
            fields.put("largeBlob", &value.to_json_value()?)?;
        }
        fields.put_optional("credProps", self.credential_properties().as_ref())?;
        if let Some(value) = self.credential_protection_policy() {
            fields.put("credentialProtectionPolicy", value.as_str())?;
        }
        fields.put_optional(
            "enforceCredentialProtectionPolicy",
            self.enforce_credential_protection_policy().as_ref(),
        )?;
        Ok(fields.into_value())
    }

    pub(crate) fn from_parts(
        credential_properties: Option<bool>,
        credential_protection_policy: Option<CredentialProtectionPolicy>,
        enforce_credential_protection_policy: Option<bool>,
    ) -> Self {
        Self {
            large_blob: None,
            credential_properties,
            credential_protection_policy,
            enforce_credential_protection_policy,
        }
    }
    pub fn with_credential_properties(mut self, value: bool) -> Self {
        self.credential_properties = Some(value);
        self
    }
    pub fn with_credential_protection_policy(mut self, value: CredentialProtectionPolicy) -> Self {
        self.credential_protection_policy = Some(value);
        self
    }
    pub fn with_enforce_credential_protection_policy(mut self, value: bool) -> Self {
        self.enforce_credential_protection_policy = Some(value);
        self
    }
    pub const fn credential_properties(&self) -> Option<bool> {
        self.credential_properties
    }
    pub fn credential_protection_policy(&self) -> Option<&CredentialProtectionPolicy> {
        self.credential_protection_policy.as_ref()
    }
    pub const fn enforce_credential_protection_policy(&self) -> Option<bool> {
        self.enforce_credential_protection_policy
    }
}
