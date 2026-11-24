use super::{
    AuthenticationExtensionsClientOutputs, AuthenticatorAssertionResponse, AuthenticatorAttachment,
    AuthenticatorAttestationResponse, ProtocolError, PublicKeyCredentialType,
};
use crate::codec::JsonObject;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicKeyCredential<R> {
    id: String,
    credential_type: PublicKeyCredentialType,
    raw_id: Vec<u8>,
    response: R,
    authenticator_attachment: Option<AuthenticatorAttachment>,
    client_extension_results: AuthenticationExtensionsClientOutputs,
}

impl PublicKeyCredential<AuthenticatorAttestationResponse> {
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut fields = JsonObject::from_bytes(bytes)?;
        let response =
            AuthenticatorAttestationResponse::from_json_value(fields.take_value("response")?)?;
        Self::from_json_object(fields, response)
    }

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        self.to_json_object(self.response.to_json_value()?)?
            .into_bytes()
    }
}

impl PublicKeyCredential<AuthenticatorAssertionResponse> {
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut fields = JsonObject::from_bytes(bytes)?;
        let response =
            AuthenticatorAssertionResponse::from_json_value(fields.take_value("response")?)?;
        Self::from_json_object(fields, response)
    }

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        self.to_json_object(self.response.to_json_value())?
            .into_bytes()
    }
}

impl<R> PublicKeyCredential<R> {
    fn from_json_object(mut fields: JsonObject, response: R) -> Result<Self, ProtocolError> {
        Ok(Self::new(
            fields.take("id")?,
            fields.take("type")?,
            fields.take_binary("rawId")?,
            response,
            fields.take_optional("authenticatorAttachment")?,
            AuthenticationExtensionsClientOutputs::from_json_value(
                fields.take_value("clientExtensionResults")?,
            )?,
        ))
    }

    fn to_json_object(&self, response: serde_json::Value) -> Result<JsonObject, ProtocolError> {
        let mut fields = JsonObject::default();
        fields.put("id", self.id())?;
        fields.put_binary("rawId", self.raw_id());
        fields.insert("response".into(), response);
        fields.put_optional("authenticatorAttachment", self.authenticator_attachment())?;
        fields.insert(
            "clientExtensionResults".into(),
            self.client_extension_results.to_json_value()?,
        );
        fields.put("type", self.credential_type())?;
        Ok(fields)
    }

    pub fn new(
        id: String,
        credential_type: PublicKeyCredentialType,
        raw_id: Vec<u8>,
        response: R,
        authenticator_attachment: Option<AuthenticatorAttachment>,
        client_extension_results: AuthenticationExtensionsClientOutputs,
    ) -> Self {
        Self {
            id,
            credential_type,
            raw_id,
            response,
            authenticator_attachment,
            client_extension_results,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn credential_type(&self) -> &PublicKeyCredentialType {
        &self.credential_type
    }

    pub fn raw_id(&self) -> &[u8] {
        &self.raw_id
    }

    pub const fn response(&self) -> &R {
        &self.response
    }

    pub const fn authenticator_attachment(&self) -> Option<&AuthenticatorAttachment> {
        self.authenticator_attachment.as_ref()
    }

    pub fn with_client_extension_results(
        mut self,
        value: AuthenticationExtensionsClientOutputs,
    ) -> Self {
        self.client_extension_results = value;
        self
    }

    pub const fn client_extension_results(&self) -> &AuthenticationExtensionsClientOutputs {
        &self.client_extension_results
    }
}
