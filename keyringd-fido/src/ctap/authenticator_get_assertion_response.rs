use crate::codec::CborItem as Value;

use super::{
    AuthenticationExtensionsAuthenticatorOutputs, CtapValue, ProtocolError,
    PublicKeyCredentialDescriptor, PublicKeyCredentialUserEntity,
};
use crate::AuthenticatorData;
use crate::codec::CborCodec;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorGetAssertionResponse {
    pub(crate) credential: Option<PublicKeyCredentialDescriptor>,
    pub(crate) authenticator_data: AuthenticatorData,
    pub(crate) signature: Vec<u8>,
    pub(crate) user: Option<PublicKeyCredentialUserEntity>,
    pub(crate) number_of_credentials: Option<u64>,
    pub(crate) user_selected: Option<bool>,
    pub(crate) large_blob_key: Option<Vec<u8>>,
    pub(crate) unsigned_extension_outputs: Option<AuthenticationExtensionsAuthenticatorOutputs>,
}

impl AuthenticatorGetAssertionResponse {
    pub fn new(
        credential: Option<PublicKeyCredentialDescriptor>,
        authenticator_data: AuthenticatorData,
        signature: Vec<u8>,
    ) -> Self {
        Self {
            credential,
            authenticator_data,
            signature,
            user: None,
            number_of_credentials: None,
            user_selected: None,
            large_blob_key: None,
            unsigned_extension_outputs: None,
        }
    }
    pub fn with_user(mut self, value: PublicKeyCredentialUserEntity) -> Self {
        self.user = Some(value);
        self
    }
    pub fn with_number_of_credentials(mut self, value: u64) -> Self {
        self.number_of_credentials = Some(value);
        self
    }
    pub fn with_user_selected(mut self, value: bool) -> Self {
        self.user_selected = Some(value);
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
    pub fn credential(&self) -> Option<&PublicKeyCredentialDescriptor> {
        self.credential.as_ref()
    }
    pub fn authenticator_data(&self) -> &AuthenticatorData {
        &self.authenticator_data
    }
    pub fn signature(&self) -> &[u8] {
        &self.signature
    }
    pub fn user(&self) -> Option<&PublicKeyCredentialUserEntity> {
        self.user.as_ref()
    }
    pub const fn number_of_credentials(&self) -> Option<u64> {
        self.number_of_credentials
    }
    pub const fn user_selected(&self) -> Option<bool> {
        self.user_selected
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
        let mut response = Self {
            credential: fields
                .remove(&1)
                .map(PublicKeyCredentialDescriptor::from_cbor_value)
                .transpose()?,
            authenticator_data: AuthenticatorData::from_bytes(&CtapValue::bytes(
                fields
                    .remove(&2)
                    .ok_or(ProtocolError::MissingCtapField(2))?,
                "authData",
            )?)?,
            signature: CtapValue::bytes(
                fields
                    .remove(&3)
                    .ok_or(ProtocolError::MissingCtapField(3))?,
                "signature",
            )?,
            user: fields
                .remove(&4)
                .map(PublicKeyCredentialUserEntity::from_cbor_value)
                .transpose()?,
            number_of_credentials: fields
                .remove(&5)
                .map(|value| CtapValue::unsigned(value, "numberOfCredentials"))
                .transpose()?,
            user_selected: fields
                .remove(&6)
                .map(|value| CtapValue::boolean(value, "userSelected"))
                .transpose()?,
            large_blob_key: fields
                .remove(&7)
                .map(|value| CtapValue::bytes(value, "largeBlobKey"))
                .transpose()?,
            unsigned_extension_outputs: fields
                .remove(&8)
                .map(AuthenticationExtensionsAuthenticatorOutputs::from_cbor_value)
                .transpose()?
                .filter(|outputs| !outputs.is_empty()),
        };
        // The prohibition on returning identifying user fields without UV is
        // an authenticator privacy requirement. A client has already received
        // those fields, so rejecting an otherwise valid assertion adds no
        // protection and harms interoperability. Retain only the user ID.
        if !response.authenticator_data.user_verified()
            && let Some(user) = &response.user
        {
            response.user = Some(PublicKeyCredentialUserEntity::from_id(user.id().to_vec()));
        }
        response.validate()?;
        Ok(response)
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;
        let mut fields = vec![
            (
                2u64.into(),
                Value::ByteString(self.authenticator_data.to_bytes()?),
            ),
            (3u64.into(), Value::ByteString(self.signature.clone())),
        ];
        if let Some(credential) = &self.credential {
            fields.push((1u64.into(), credential.to_cbor_value()?));
        }
        if let Some(value) = &self.user {
            fields.push((4u64.into(), value.to_cbor_value()?));
        }
        if let Some(value) = self.number_of_credentials {
            fields.push((5u64.into(), value.into()));
        }
        if let Some(value) = self.user_selected {
            fields.push((6u64.into(), Value::bool_value(value)));
        }
        if let Some(value) = &self.large_blob_key {
            fields.push((7u64.into(), Value::ByteString(value.clone())));
        }
        if let Some(value) = &self.unsigned_extension_outputs {
            fields.push((8u64.into(), value.to_cbor_value()?));
        }
        Ok(CborCodec::encode(Value::Map(fields))?)
    }

    fn validate(&self) -> Result<(), ProtocolError> {
        if self.number_of_credentials == Some(0) {
            return Err(ProtocolError::MustBePositive {
                field: "numberOfCredentials",
            });
        }
        if self.number_of_credentials.is_some_and(|count| count > 1) && self.user_selected.is_some()
        {
            return Err(ProtocolError::UnexpectedType {
                field: "userSelected",
                expected: "absent when numberOfCredentials is greater than one",
            });
        }
        if !self.authenticator_data.user_verified()
            && self
                .user
                .as_ref()
                .is_some_and(|user| user.name().is_some() || user.display_name().is_some())
        {
            return Err(ProtocolError::UnexpectedType {
                field: "user",
                expected: "an ID-only user entity when user verification was not performed",
            });
        }
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
