use std::collections::BTreeMap;

use crate::codec::CborItem as Value;

use crate::codec::CborValue;
use crate::{EncodedCborValue, ProtocolError};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuthenticationExtensionsAuthenticatorInputs {
    values: BTreeMap<String, EncodedCborValue>,
}

impl AuthenticationExtensionsAuthenticatorInputs {
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

    pub(crate) fn from_cbor_value(
        value: Value,
        field: &'static str,
    ) -> Result<Self, ProtocolError> {
        let Value::Map(values) = value else {
            return Err(CborValue::unexpected(field, "a map"));
        };
        let mut result = BTreeMap::new();
        for (key, value) in values {
            let key = CborValue::text(key, field)?;
            if result
                .insert(key, EncodedCborValue::from_value(value)?)
                .is_some()
            {
                return Err(ProtocolError::DuplicateValue { field });
            }
        }
        Ok(Self { values: result })
    }

    pub(crate) fn to_cbor_value(&self) -> Result<Value, ProtocolError> {
        self.values
            .iter()
            .map(|(key, value)| Ok((Value::TextString(key.clone()), value.to_value()?)))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Map)
    }
}
