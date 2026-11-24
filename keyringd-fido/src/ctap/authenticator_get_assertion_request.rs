use std::collections::BTreeMap;

use crate::codec::CborItem as Value;

use super::{
    AuthenticationExtensionsAuthenticatorInputs, ClientDataHash, CtapValue, ProtocolError,
    PublicKeyCredentialDescriptor,
};
use crate::codec::CborCodec;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorGetAssertionRequest {
    pub(crate) rp_id: String,
    pub(crate) client_data_hash: ClientDataHash,
    pub(crate) allow_list: Option<Vec<PublicKeyCredentialDescriptor>>,
    pub(crate) extensions: Option<AuthenticationExtensionsAuthenticatorInputs>,
    pub(crate) options: Option<BTreeMap<String, bool>>,
    pub(crate) pin_uv_auth_param: Option<Vec<u8>>,
    pub(crate) pin_uv_auth_protocol: Option<u64>,
}

impl AuthenticatorGetAssertionRequest {
    pub fn new(rp_id: impl Into<String>, client_data_hash: ClientDataHash) -> Self {
        Self {
            rp_id: rp_id.into(),
            client_data_hash,
            allow_list: None,
            extensions: None,
            options: None,
            pin_uv_auth_param: None,
            pin_uv_auth_protocol: None,
        }
    }
    pub fn with_allow_list(mut self, value: Vec<PublicKeyCredentialDescriptor>) -> Self {
        self.allow_list = (!value.is_empty()).then_some(value);
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
    pub fn rp_id(&self) -> &str {
        &self.rp_id
    }
    pub const fn client_data_hash(&self) -> &ClientDataHash {
        &self.client_data_hash
    }
    pub fn allow_list(&self) -> Option<&[PublicKeyCredentialDescriptor]> {
        self.allow_list.as_deref()
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

    pub fn from_cbor_bytes(encoded: &[u8]) -> Result<Self, ProtocolError> {
        let mut fields = CtapValue::ctap_map(CborCodec::decode(encoded)?)?;
        let hash = CtapValue::bytes(
            fields
                .remove(&2)
                .ok_or(ProtocolError::MissingCtapField(2))?,
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
            rp_id: CtapValue::text(
                fields
                    .remove(&1)
                    .ok_or(ProtocolError::MissingCtapField(1))?,
                "rpId",
            )?,
            client_data_hash: ClientDataHash::new(hash.try_into().expect("validated hash length")),
            allow_list: fields
                .remove(&3)
                .map(|value| PublicKeyCredentialDescriptor::vec_from_cbor_value(value, "allowList"))
                .transpose()?,
            extensions: fields
                .remove(&4)
                .map(|value| {
                    AuthenticationExtensionsAuthenticatorInputs::from_cbor_value(
                        value,
                        "extensions",
                    )
                })
                .transpose()?,
            options: fields.remove(&5).map(CtapValue::boolean_map).transpose()?,
            pin_uv_auth_param: fields
                .remove(&6)
                .map(|value| CtapValue::bytes(value, "pinUvAuthParam"))
                .transpose()?,
            pin_uv_auth_protocol: fields
                .remove(&7)
                .map(|value| CtapValue::unsigned(value, "pinUvAuthProtocol"))
                .transpose()?,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;
        let mut fields = vec![
            (1u64.into(), Value::TextString(self.rp_id.clone())),
            (
                2u64.into(),
                Value::ByteString(self.client_data_hash.as_bytes().to_vec()),
            ),
        ];
        if let Some(value) = &self.allow_list {
            fields.push((
                3u64.into(),
                PublicKeyCredentialDescriptor::vec_to_cbor_value(value)?,
            ));
        }
        if let Some(value) = &self.extensions {
            fields.push((4u64.into(), value.to_cbor_value()?));
        }
        if let Some(value) = &self.options {
            fields.push((5u64.into(), option_value(value)));
        }
        if let Some(value) = &self.pin_uv_auth_param {
            fields.push((6u64.into(), Value::ByteString(value.clone())));
        }
        if let Some(value) = self.pin_uv_auth_protocol {
            fields.push((7u64.into(), value.into()));
        }
        Ok(CborCodec::encode(Value::Map(fields))?)
    }

    fn validate(&self) -> Result<(), ProtocolError> {
        if self.allow_list.as_ref().is_some_and(Vec::is_empty) {
            return Err(ProtocolError::EmptyValue { field: "allowList" });
        }
        if self.pin_uv_auth_param.is_some() && self.pin_uv_auth_protocol.is_none() {
            return Err(ProtocolError::UnexpectedType {
                field: "pinUvAuthParam/pinUvAuthProtocol",
                expected: "pinUvAuthProtocol whenever pinUvAuthParam is present",
            });
        }
        if self
            .options
            .as_ref()
            .is_some_and(|options| options.contains_key("rk"))
        {
            return Err(ProtocolError::InvalidValue {
                field: "options.rk",
                expected: "absent from GetAssertion requests",
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
        Ok(())
    }
}

pub(super) fn option_value(values: &BTreeMap<String, bool>) -> Value {
    Value::Map(
        values
            .iter()
            .map(|(key, value)| (Value::TextString(key.clone()), Value::bool_value(*value)))
            .collect(),
    )
}
