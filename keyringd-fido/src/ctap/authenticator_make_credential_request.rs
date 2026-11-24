use std::collections::{BTreeMap, HashSet};

use crate::codec::CborItem as Value;

use super::authenticator_get_assertion_request::option_value;
use super::{
    AuthenticationExtensionsAuthenticatorInputs, ClientDataHash, CtapValue, ProtocolError,
    PublicKeyCredentialDescriptor, PublicKeyCredentialParameters, PublicKeyCredentialRpEntity,
    PublicKeyCredentialUserEntity,
};
use crate::codec::CborCodec;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorMakeCredentialRequest {
    pub(crate) client_data_hash: ClientDataHash,
    pub(crate) relying_party: PublicKeyCredentialRpEntity,
    pub(crate) user: PublicKeyCredentialUserEntity,
    pub(crate) public_key_credential_parameters: Vec<PublicKeyCredentialParameters>,
    pub(crate) exclude_list: Option<Vec<PublicKeyCredentialDescriptor>>,
    pub(crate) extensions: Option<AuthenticationExtensionsAuthenticatorInputs>,
    pub(crate) options: Option<BTreeMap<String, bool>>,
    pub(crate) pin_uv_auth_param: Option<Vec<u8>>,
    pub(crate) pin_uv_auth_protocol: Option<u64>,
    pub(crate) enterprise_attestation: Option<u64>,
    pub(crate) attestation_formats_preference: Option<Vec<String>>,
}

impl AuthenticatorMakeCredentialRequest {
    pub fn new(
        client_data_hash: ClientDataHash,
        relying_party: PublicKeyCredentialRpEntity,
        user: PublicKeyCredentialUserEntity,
        public_key_credential_parameters: Vec<PublicKeyCredentialParameters>,
    ) -> Self {
        Self {
            client_data_hash,
            relying_party,
            user,
            public_key_credential_parameters,
            exclude_list: None,
            extensions: None,
            options: None,
            pin_uv_auth_param: None,
            pin_uv_auth_protocol: None,
            enterprise_attestation: None,
            attestation_formats_preference: None,
        }
    }
    pub fn with_exclude_list(mut self, value: Vec<PublicKeyCredentialDescriptor>) -> Self {
        self.exclude_list = (!value.is_empty()).then_some(value);
        self
    }
    pub fn with_extensions(mut self, value: AuthenticationExtensionsAuthenticatorInputs) -> Self {
        self.extensions = Some(value);
        self
    }
    pub fn with_options(mut self, value: BTreeMap<String, bool>) -> Self {
        self.options = Some(value);
        self
    }
    pub fn with_option(mut self, name: impl Into<String>, value: bool) -> Self {
        self.options
            .get_or_insert_default()
            .insert(name.into(), value);
        self
    }
    pub fn with_pin_uv_auth(mut self, parameter: Vec<u8>, protocol: u64) -> Self {
        self.pin_uv_auth_param = Some(parameter);
        self.pin_uv_auth_protocol = Some(protocol);
        self
    }
    pub fn with_enterprise_attestation(mut self, value: u64) -> Self {
        self.enterprise_attestation = Some(value);
        self
    }
    pub fn with_attestation_formats_preference(mut self, value: Vec<String>) -> Self {
        self.attestation_formats_preference = Some(value);
        self
    }
    pub fn relying_party(&self) -> &PublicKeyCredentialRpEntity {
        &self.relying_party
    }
    pub fn user(&self) -> &PublicKeyCredentialUserEntity {
        &self.user
    }
    pub fn public_key_credential_parameters(&self) -> &[PublicKeyCredentialParameters] {
        &self.public_key_credential_parameters
    }
    pub fn exclude_list(&self) -> Option<&[PublicKeyCredentialDescriptor]> {
        self.exclude_list.as_deref()
    }
    pub fn extensions(&self) -> Option<&AuthenticationExtensionsAuthenticatorInputs> {
        self.extensions.as_ref()
    }
    pub fn options(&self) -> Option<&BTreeMap<String, bool>> {
        self.options.as_ref()
    }
    pub fn pin_uv_auth_param(&self) -> Option<&[u8]> {
        self.pin_uv_auth_param.as_deref()
    }
    pub const fn pin_uv_auth_protocol(&self) -> Option<u64> {
        self.pin_uv_auth_protocol
    }
    pub const fn enterprise_attestation(&self) -> Option<u64> {
        self.enterprise_attestation
    }
    pub fn attestation_formats_preference(&self) -> Option<&[String]> {
        self.attestation_formats_preference.as_deref()
    }
    pub const fn client_data_hash(&self) -> &ClientDataHash {
        &self.client_data_hash
    }

    pub fn from_cbor_bytes(encoded: &[u8]) -> Result<Self, ProtocolError> {
        let mut fields = CtapValue::ctap_map(CborCodec::decode(encoded)?)?;
        let hash = CtapValue::bytes(
            fields
                .remove(&1)
                .ok_or(ProtocolError::MissingCtapField(1))?,
            "clientDataHash",
        )?;
        if hash.len() != ClientDataHash::LENGTH {
            return Err(ProtocolError::InvalidLength {
                field: "clientDataHash",
                expected: ClientDataHash::LENGTH,
                actual: hash.len(),
            });
        }
        let request = Self {
            client_data_hash: ClientDataHash::new(hash.try_into().expect("validated hash length")),
            relying_party: PublicKeyCredentialRpEntity::from_cbor_value(
                fields
                    .remove(&2)
                    .ok_or(ProtocolError::MissingCtapField(2))?,
            )?,
            user: PublicKeyCredentialUserEntity::from_cbor_value(
                fields
                    .remove(&3)
                    .ok_or(ProtocolError::MissingCtapField(3))?,
            )?,
            public_key_credential_parameters: PublicKeyCredentialParameters::vec_from_value(
                fields
                    .remove(&4)
                    .ok_or(ProtocolError::MissingCtapField(4))?,
            )?,
            exclude_list: fields
                .remove(&5)
                .map(|value| {
                    PublicKeyCredentialDescriptor::vec_from_cbor_value(value, "excludeList")
                })
                .transpose()?,
            extensions: fields
                .remove(&6)
                .map(|value| {
                    AuthenticationExtensionsAuthenticatorInputs::from_cbor_value(
                        value,
                        "extensions",
                    )
                })
                .transpose()?,
            options: fields.remove(&7).map(CtapValue::boolean_map).transpose()?,
            pin_uv_auth_param: take_bytes(&mut fields, 8, "pinUvAuthParam")?,
            pin_uv_auth_protocol: take_unsigned(&mut fields, 9, "pinUvAuthProtocol")?,
            enterprise_attestation: take_unsigned(&mut fields, 10, "enterpriseAttestation")?,
            attestation_formats_preference: fields
                .remove(&11)
                .map(|value| CtapValue::text_array(value, "attestationFormatsPreference", true))
                .transpose()?,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;
        let mut fields = vec![
            (
                1u64.into(),
                Value::ByteString(self.client_data_hash.as_bytes().to_vec()),
            ),
            (2u64.into(), self.relying_party.to_cbor_value()?),
            (3u64.into(), self.user.to_cbor_value()?),
            (
                4u64.into(),
                PublicKeyCredentialParameters::vec_to_value(
                    &self.public_key_credential_parameters,
                )?,
            ),
        ];
        if let Some(value) = &self.exclude_list {
            fields.push((
                5u64.into(),
                PublicKeyCredentialDescriptor::vec_to_cbor_value(value)?,
            ));
        }
        if let Some(value) = &self.extensions {
            fields.push((6u64.into(), value.to_cbor_value()?));
        }
        if let Some(value) = &self.options {
            fields.push((7u64.into(), option_value(value)));
        }
        push_bytes(&mut fields, 8, self.pin_uv_auth_param.as_deref());
        push_unsigned(&mut fields, 9, self.pin_uv_auth_protocol);
        push_unsigned(&mut fields, 10, self.enterprise_attestation);
        if let Some(value) = &self.attestation_formats_preference {
            fields.push((
                11u64.into(),
                Value::Array(value.iter().cloned().map(Value::TextString).collect()),
            ));
        }
        Ok(CborCodec::encode(Value::Map(fields))?)
    }

    fn validate(&self) -> Result<(), ProtocolError> {
        if self.public_key_credential_parameters.is_empty() {
            return Err(ProtocolError::EmptyValue {
                field: "pubKeyCredParams",
            });
        }
        let mut algorithms = HashSet::with_capacity(self.public_key_credential_parameters.len());
        if !self
            .public_key_credential_parameters
            .iter()
            .all(|parameter| {
                algorithms.insert((parameter.credential_type().as_str(), parameter.algorithm()))
            })
        {
            return Err(ProtocolError::DuplicateValue {
                field: "pubKeyCredParams",
            });
        }
        if self.exclude_list.as_ref().is_some_and(Vec::is_empty) {
            return Err(ProtocolError::EmptyValue {
                field: "excludeList",
            });
        }
        if self.pin_uv_auth_param.is_some() && self.pin_uv_auth_protocol.is_none() {
            return Err(ProtocolError::UnexpectedType {
                field: "pinUvAuthParam/pinUvAuthProtocol",
                expected: "pinUvAuthProtocol whenever pinUvAuthParam is present",
            });
        }
        if self.options.as_ref().and_then(|options| options.get("up")) == Some(&false) {
            return Err(ProtocolError::InvalidValue {
                field: "options.up",
                expected: "true when present",
            });
        }
        if self.pin_uv_auth_param.is_some()
            && self
                .options
                .as_ref()
                .is_some_and(|options| options.contains_key("uv"))
        {
            return Err(ProtocolError::InvalidValue {
                field: "options.uv/pinUvAuthParam",
                expected: "at most one to be present",
            });
        }
        if self
            .enterprise_attestation
            .is_some_and(|value| !matches!(value, 1 | 2))
        {
            return Err(ProtocolError::InvalidValue {
                field: "enterpriseAttestation",
                expected: "1 or 2",
            });
        }
        Ok(())
    }
}

fn take_bytes(
    fields: &mut BTreeMap<u64, Value>,
    key: u64,
    field: &'static str,
) -> Result<Option<Vec<u8>>, ProtocolError> {
    fields
        .remove(&key)
        .map(|value| CtapValue::bytes(value, field))
        .transpose()
}

fn take_unsigned(
    fields: &mut BTreeMap<u64, Value>,
    key: u64,
    field: &'static str,
) -> Result<Option<u64>, ProtocolError> {
    fields
        .remove(&key)
        .map(|value| CtapValue::unsigned(value, field))
        .transpose()
}

fn push_bytes(fields: &mut Vec<(Value, Value)>, key: u64, value: Option<&[u8]>) {
    if let Some(value) = value {
        fields.push((key.into(), Value::ByteString(value.to_vec())));
    }
}

fn push_unsigned(fields: &mut Vec<(Value, Value)>, key: u64, value: Option<u64>) {
    if let Some(value) = value {
        fields.push((key.into(), value.into()));
    }
}
