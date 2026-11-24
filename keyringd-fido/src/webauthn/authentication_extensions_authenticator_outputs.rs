use std::collections::BTreeMap;

use crate::codec::CborItem as Value;

use crate::codec::CborCodec;
use crate::{EncodedCborValue, ProtocolError};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuthenticationExtensionsAuthenticatorOutputs {
    values: BTreeMap<String, EncodedCborValue>,
}

impl AuthenticationExtensionsAuthenticatorOutputs {
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn with_value(mut self, name: impl Into<String>, value: EncodedCborValue) -> Self {
        self.values.insert(name.into(), value);
        self
    }

    pub fn values(&self) -> &BTreeMap<String, EncodedCborValue> {
        &self.values
    }

    pub fn from_cbor_bytes(encoded: &[u8]) -> Result<Self, ProtocolError> {
        Self::from_cbor_value(CborCodec::decode(encoded)?)
    }

    pub(crate) fn from_cbor_value(value: Value) -> Result<Self, ProtocolError> {
        let Value::Map(entries) = value else {
            return Err(ProtocolError::InvalidAuthenticatorExtensions);
        };
        let mut values = BTreeMap::new();
        for (name, value) in entries {
            let Value::TextString(name) = name else {
                return Err(ProtocolError::InvalidAuthenticatorExtensions);
            };
            if values
                .insert(name, EncodedCborValue::from_value(value)?)
                .is_some()
            {
                return Err(ProtocolError::DuplicateValue {
                    field: "authenticator extension output",
                });
            }
        }
        Ok(Self { values })
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(CborCodec::encode(self.to_cbor_value()?)?)
    }

    pub(crate) fn to_cbor_value(&self) -> Result<Value, ProtocolError> {
        Ok(Value::Map(
            self.values
                .iter()
                .map(|(name, value)| Ok((Value::TextString(name.clone()), value.to_value()?)))
                .collect::<Result<_, ProtocolError>>()?,
        ))
    }
}
