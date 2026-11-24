use super::{
    AttestationConveyancePreference, AuthenticationExtensionsClientInputs,
    AuthenticatorSelectionCriteria, ProtocolError, PublicKeyCredentialDescriptor,
    PublicKeyCredentialHint, PublicKeyCredentialParameters, PublicKeyCredentialRpEntity,
    PublicKeyCredentialUserEntity,
};
use crate::codec::{JsonObject, JsonValue};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicKeyCredentialCreationOptions {
    rp: PublicKeyCredentialRpEntity,
    user: PublicKeyCredentialUserEntity,
    challenge: Vec<u8>,
    public_key_credential_parameters: Vec<PublicKeyCredentialParameters>,
    timeout: Option<u32>,
    exclude_credentials: Vec<PublicKeyCredentialDescriptor>,
    authenticator_selection: Option<AuthenticatorSelectionCriteria>,
    hints: Vec<PublicKeyCredentialHint>,
    attestation: AttestationConveyancePreference,
    attestation_formats: Vec<String>,
    extensions: Option<AuthenticationExtensionsClientInputs>,
}

impl PublicKeyCredentialCreationOptions {
    /// Validates user-account constraints specific to WebAuthn credential creation.
    /// Client policies and authenticator capabilities are checked by the caller.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if let Some(input) = self.extensions().and_then(|e| e.large_blob()) {
            input.validate_registration()?;
        }
        self.user.validate_webauthn()
    }

    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut fields = JsonObject::from_bytes(bytes)?;
        Ok(Self::from_parts(
            fields.take("rp")?,
            PublicKeyCredentialUserEntity::from_json_value(fields.take_value("user")?)?,
            fields.take_binary("challenge")?,
            fields.take("pubKeyCredParams")?,
            fields.take_optional("timeout")?,
            PublicKeyCredentialDescriptor::vec_from_json_value(
                fields.remove("excludeCredentials"),
            )?,
            fields.take_optional("authenticatorSelection")?,
            fields.take_or_default("hints")?,
            fields
                .remove("attestation")
                .map(JsonValue::decode)
                .transpose()?
                .unwrap_or(AttestationConveyancePreference::None),
            fields.take_or_default("attestationFormats")?,
            fields
                .remove("extensions")
                .map(AuthenticationExtensionsClientInputs::from_json_value)
                .transpose()?,
        ))
    }

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut fields = JsonObject::default();
        fields.put("rp", self.rp())?;
        fields.insert("user".into(), self.user().to_json_value()?);
        fields.put_binary("challenge", self.challenge());
        fields.put("pubKeyCredParams", self.public_key_credential_parameters())?;
        fields.put_optional("timeout", self.timeout().as_ref())?;
        fields.insert(
            "excludeCredentials".into(),
            serde_json::Value::Array(
                self.exclude_credentials()
                    .iter()
                    .map(PublicKeyCredentialDescriptor::to_json_value)
                    .collect::<Result<_, _>>()?,
            ),
        );
        fields.put_optional("authenticatorSelection", self.authenticator_selection())?;
        fields.put("hints", self.hints())?;
        fields.put("attestation", self.attestation())?;
        fields.put("attestationFormats", self.attestation_formats())?;
        if let Some(value) = self.extensions() {
            fields.insert("extensions".into(), value.to_json_value()?);
        }
        fields.into_bytes()
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_parts(
        rp: PublicKeyCredentialRpEntity,
        user: PublicKeyCredentialUserEntity,
        challenge: Vec<u8>,
        public_key_credential_parameters: Vec<PublicKeyCredentialParameters>,
        timeout: Option<u32>,
        exclude_credentials: Vec<PublicKeyCredentialDescriptor>,
        authenticator_selection: Option<AuthenticatorSelectionCriteria>,
        hints: Vec<PublicKeyCredentialHint>,
        attestation: AttestationConveyancePreference,
        attestation_formats: Vec<String>,
        extensions: Option<AuthenticationExtensionsClientInputs>,
    ) -> Self {
        Self {
            rp,
            user,
            challenge,
            public_key_credential_parameters,
            timeout,
            exclude_credentials,
            authenticator_selection,
            hints,
            attestation,
            attestation_formats,
            extensions,
        }
    }

    pub fn new(
        rp: PublicKeyCredentialRpEntity,
        user: PublicKeyCredentialUserEntity,
        challenge: Vec<u8>,
        public_key_credential_parameters: Vec<PublicKeyCredentialParameters>,
    ) -> Self {
        Self {
            rp,
            user,
            challenge,
            public_key_credential_parameters,
            timeout: None,
            exclude_credentials: Vec::new(),
            authenticator_selection: None,
            hints: Vec::new(),
            attestation: AttestationConveyancePreference::None,
            attestation_formats: Vec::new(),
            extensions: None,
        }
    }

    pub fn with_timeout(mut self, timeout: u32) -> Self {
        self.timeout = Some(timeout);
        self
    }
    pub fn with_public_key_credential_parameters(
        mut self,
        value: Vec<PublicKeyCredentialParameters>,
    ) -> Self {
        self.public_key_credential_parameters = value;
        self
    }
    pub fn with_exclude_credentials(mut self, value: Vec<PublicKeyCredentialDescriptor>) -> Self {
        self.exclude_credentials = value;
        self
    }
    pub fn with_authenticator_selection(mut self, value: AuthenticatorSelectionCriteria) -> Self {
        self.authenticator_selection = Some(value);
        self
    }
    pub fn with_hints(mut self, value: Vec<PublicKeyCredentialHint>) -> Self {
        self.hints = value;
        self
    }
    pub fn with_attestation(mut self, value: AttestationConveyancePreference) -> Self {
        self.attestation = value;
        self
    }
    pub fn with_attestation_formats(mut self, value: Vec<String>) -> Self {
        self.attestation_formats = value;
        self
    }
    pub fn with_extensions(mut self, value: AuthenticationExtensionsClientInputs) -> Self {
        self.extensions = Some(value);
        self
    }
    pub fn rp(&self) -> &PublicKeyCredentialRpEntity {
        &self.rp
    }
    pub fn user(&self) -> &PublicKeyCredentialUserEntity {
        &self.user
    }
    pub fn challenge(&self) -> &[u8] {
        &self.challenge
    }
    pub fn public_key_credential_parameters(&self) -> &[PublicKeyCredentialParameters] {
        &self.public_key_credential_parameters
    }
    pub const fn timeout(&self) -> Option<u32> {
        self.timeout
    }
    pub fn exclude_credentials(&self) -> &[PublicKeyCredentialDescriptor] {
        &self.exclude_credentials
    }
    pub fn authenticator_selection(&self) -> Option<&AuthenticatorSelectionCriteria> {
        self.authenticator_selection.as_ref()
    }
    pub fn hints(&self) -> &[PublicKeyCredentialHint] {
        &self.hints
    }
    pub fn attestation(&self) -> &AttestationConveyancePreference {
        &self.attestation
    }
    pub fn attestation_formats(&self) -> &[String] {
        &self.attestation_formats
    }
    pub fn extensions(&self) -> Option<&AuthenticationExtensionsClientInputs> {
        self.extensions.as_ref()
    }
}
