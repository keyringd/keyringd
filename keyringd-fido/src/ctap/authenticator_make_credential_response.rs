use crate::codec::CborItem as Value;

use super::{AuthenticationExtensionsAuthenticatorOutputs, CtapValue, ProtocolError};
use crate::codec::CborCodec;
use crate::{AttestationStatement, AuthenticatorData};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorMakeCredentialResponse {
    pub(crate) format: String,
    pub(crate) authenticator_data: AuthenticatorData,
    pub(crate) attestation_statement: Option<AttestationStatement>,
    pub(crate) enterprise_attestation: Option<bool>,
    pub(crate) large_blob_key: Option<Vec<u8>>,
    pub(crate) unsigned_extension_outputs: Option<AuthenticationExtensionsAuthenticatorOutputs>,
}

impl AuthenticatorMakeCredentialResponse {
    pub fn new(
        format: impl Into<String>,
        authenticator_data: AuthenticatorData,
        attestation_statement: AttestationStatement,
    ) -> Self {
        Self {
            format: format.into(),
            authenticator_data,
            attestation_statement: Some(attestation_statement),
            enterprise_attestation: None,
            large_blob_key: None,
            unsigned_extension_outputs: None,
        }
    }
    pub fn with_enterprise_attestation(mut self, value: bool) -> Self {
        self.enterprise_attestation = Some(value);
        self
    }
    pub fn with_large_blob_key(mut self, value: Vec<u8>) -> Self {
        self.large_blob_key = Some(value);
        self
    }
    pub fn with_unsigned_extension_outputs(
        mut self,
        value: AuthenticationExtensionsAuthenticatorOutputs,
    ) -> Self {
        self.unsigned_extension_outputs = Some(value);
        self
    }
    pub fn format(&self) -> &str {
        &self.format
    }
    pub fn authenticator_data(&self) -> &AuthenticatorData {
        &self.authenticator_data
    }
    pub fn attestation_statement(&self) -> Option<&AttestationStatement> {
        self.attestation_statement.as_ref()
    }
    pub const fn enterprise_attestation(&self) -> Option<bool> {
        self.enterprise_attestation
    }
    pub fn large_blob_key(&self) -> Option<&[u8]> {
        self.large_blob_key.as_deref()
    }
    pub fn unsigned_extension_outputs(
        &self,
    ) -> Option<&AuthenticationExtensionsAuthenticatorOutputs> {
        self.unsigned_extension_outputs.as_ref()
    }
    pub fn from_cbor_bytes(encoded: &[u8]) -> Result<Self, ProtocolError> {
        let mut fields = CtapValue::ctap_map(CborCodec::decode(encoded)?)?;
        let format = CtapValue::text(
            fields
                .remove(&1)
                .ok_or(ProtocolError::MissingCtapField(1))?,
            "fmt",
        )?;
        let attestation_statement = fields
            .remove(&3)
            .map(|value| AttestationStatement::from_cbor_value(&format, value))
            .transpose()?;
        let response = Self {
            format,
            authenticator_data: AuthenticatorData::from_bytes(&CtapValue::bytes(
                fields
                    .remove(&2)
                    .ok_or(ProtocolError::MissingCtapField(2))?,
                "authData",
            )?)?,
            attestation_statement,
            enterprise_attestation: fields
                .remove(&4)
                .map(|value| CtapValue::boolean(value, "epAtt"))
                .transpose()?,
            large_blob_key: fields
                .remove(&5)
                .map(|value| CtapValue::bytes(value, "largeBlobKey"))
                .transpose()?,
            unsigned_extension_outputs: fields
                .remove(&6)
                .map(AuthenticationExtensionsAuthenticatorOutputs::from_cbor_value)
                .transpose()?
                .filter(|outputs| !outputs.is_empty()),
        };
        response.validate()?;
        Ok(response)
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;
        let mut fields = vec![
            (1u64.into(), Value::TextString(self.format.clone())),
            (
                2u64.into(),
                Value::ByteString(self.authenticator_data.to_bytes()?),
            ),
        ];
        if let Some(statement) = &self.attestation_statement {
            if statement.format() != self.format {
                return Err(ProtocolError::UnexpectedType {
                    field: "fmt/attStmt",
                    expected: "matching attestation formats",
                });
            }
            fields.push((3u64.into(), statement.to_cbor_value()?));
        }
        if let Some(value) = self.enterprise_attestation {
            fields.push((4u64.into(), Value::bool_value(value)));
        }
        if let Some(value) = &self.large_blob_key {
            fields.push((5u64.into(), Value::ByteString(value.clone())));
        }
        if let Some(value) = &self.unsigned_extension_outputs {
            fields.push((6u64.into(), value.to_cbor_value()?));
        }
        Ok(CborCodec::encode(Value::Map(fields))?)
    }

    fn validate(&self) -> Result<(), ProtocolError> {
        if let Some(key) = &self.large_blob_key
            && key.len() != 32
        {
            return Err(ProtocolError::InvalidLength {
                field: "largeBlobKey",
                expected: 32,
                actual: key.len(),
            });
        }
        Ok(())
    }
}
