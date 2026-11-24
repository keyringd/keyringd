use super::{
    AuthenticationExtensionsClientInputs, ProtocolError, PublicKeyCredentialDescriptor,
    PublicKeyCredentialHint, UserVerificationRequirement,
};
use crate::codec::{JsonObject, JsonValue};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicKeyCredentialRequestOptions {
    challenge: Vec<u8>,
    timeout: Option<u32>,
    relying_party_id: Option<String>,
    allow_credentials: Vec<PublicKeyCredentialDescriptor>,
    user_verification: UserVerificationRequirement,
    hints: Vec<PublicKeyCredentialHint>,
    extensions: Option<AuthenticationExtensionsClientInputs>,
}

impl PublicKeyCredentialRequestOptions {
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut fields = JsonObject::from_bytes(bytes)?;
        Ok(Self::from_parts(
            fields.take_binary("challenge")?,
            fields.take_optional("timeout")?,
            fields.take_optional("rpId")?,
            PublicKeyCredentialDescriptor::vec_from_json_value(fields.remove("allowCredentials"))?,
            fields
                .remove("userVerification")
                .map(JsonValue::decode)
                .transpose()?
                .unwrap_or(UserVerificationRequirement::Preferred),
            fields.take_or_default("hints")?,
            fields
                .remove("extensions")
                .map(AuthenticationExtensionsClientInputs::from_json_value)
                .transpose()?,
        ))
    }

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut fields = JsonObject::default();
        fields.put_binary("challenge", self.challenge());
        fields.put_optional("timeout", self.timeout().as_ref())?;
        fields.put_optional("rpId", self.relying_party_id())?;
        fields.insert(
            "allowCredentials".into(),
            serde_json::Value::Array(
                self.allow_credentials()
                    .iter()
                    .map(PublicKeyCredentialDescriptor::to_json_value)
                    .collect::<Result<_, _>>()?,
            ),
        );
        fields.put("userVerification", self.user_verification())?;
        fields.put("hints", self.hints())?;
        if let Some(value) = self.extensions() {
            fields.insert("extensions".into(), value.to_json_value()?);
        }
        fields.into_bytes()
    }

    pub(crate) fn from_parts(
        challenge: Vec<u8>,
        timeout: Option<u32>,
        relying_party_id: Option<String>,
        allow_credentials: Vec<PublicKeyCredentialDescriptor>,
        user_verification: UserVerificationRequirement,
        hints: Vec<PublicKeyCredentialHint>,
        extensions: Option<AuthenticationExtensionsClientInputs>,
    ) -> Self {
        Self {
            challenge,
            timeout,
            relying_party_id,
            allow_credentials,
            user_verification,
            hints,
            extensions,
        }
    }

    pub fn new(challenge: Vec<u8>) -> Self {
        Self {
            challenge,
            timeout: None,
            relying_party_id: None,
            allow_credentials: Vec::new(),
            user_verification: UserVerificationRequirement::Preferred,
            hints: Vec::new(),
            extensions: None,
        }
    }

    pub fn with_timeout(mut self, timeout: u32) -> Self {
        self.timeout = Some(timeout);
        self
    }
    pub fn with_relying_party_id(mut self, value: impl Into<String>) -> Self {
        self.relying_party_id = Some(value.into());
        self
    }
    pub fn with_allow_credentials(mut self, value: Vec<PublicKeyCredentialDescriptor>) -> Self {
        self.allow_credentials = value;
        self
    }
    pub fn with_user_verification(mut self, value: UserVerificationRequirement) -> Self {
        self.user_verification = value;
        self
    }
    pub fn with_hints(mut self, value: Vec<PublicKeyCredentialHint>) -> Self {
        self.hints = value;
        self
    }
    pub fn with_extensions(mut self, value: AuthenticationExtensionsClientInputs) -> Self {
        self.extensions = Some(value);
        self
    }
    pub fn challenge(&self) -> &[u8] {
        &self.challenge
    }
    pub const fn timeout(&self) -> Option<u32> {
        self.timeout
    }
    pub fn relying_party_id(&self) -> Option<&str> {
        self.relying_party_id.as_deref()
    }
    pub fn allow_credentials(&self) -> &[PublicKeyCredentialDescriptor] {
        &self.allow_credentials
    }
    pub fn user_verification(&self) -> &UserVerificationRequirement {
        &self.user_verification
    }
    pub fn hints(&self) -> &[PublicKeyCredentialHint] {
        &self.hints
    }
    pub fn extensions(&self) -> Option<&AuthenticationExtensionsClientInputs> {
        self.extensions.as_ref()
    }
}
