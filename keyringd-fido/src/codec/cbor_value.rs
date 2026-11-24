use std::collections::{BTreeMap, HashSet};

use crate::codec::{CborItem as Value, CborSimpleValue as SimpleValue};

use crate::ProtocolError;

pub(crate) struct CborValue;

impl CborValue {
    pub(crate) fn ctap_map(value: Value) -> Result<BTreeMap<u64, Value>, ProtocolError> {
        let Value::Map(entries) = value else {
            return Err(Self::unexpected("CTAP message", "a map"));
        };
        let mut fields = BTreeMap::new();
        for (key, value) in entries {
            let key = Self::unsigned(key, "CTAP map key")?;
            if fields.insert(key, value).is_some() {
                return Err(ProtocolError::DuplicateValue {
                    field: "CTAP map key",
                });
            }
        }
        Ok(fields)
    }

    pub(crate) fn text(value: Value, field: &'static str) -> Result<String, ProtocolError> {
        match value {
            Value::TextString(value) => Ok(value),
            _ => Err(Self::unexpected(field, "a text string")),
        }
    }

    pub(crate) fn bytes(value: Value, field: &'static str) -> Result<Vec<u8>, ProtocolError> {
        match value {
            Value::ByteString(value) => Ok(value),
            _ => Err(Self::unexpected(field, "a byte string")),
        }
    }

    pub(crate) fn unsigned(value: Value, field: &'static str) -> Result<u64, ProtocolError> {
        match value {
            Value::Unsigned(value) => Ok(value),
            _ => Err(Self::unexpected(field, "an unsigned integer")),
        }
    }

    pub(crate) fn boolean(value: Value, field: &'static str) -> Result<bool, ProtocolError> {
        match value {
            Value::Simple(SimpleValue::TrueValue) => Ok(true),
            Value::Simple(SimpleValue::FalseValue) => Ok(false),
            _ => Err(Self::unexpected(field, "a boolean")),
        }
    }

    pub(crate) fn text_array(
        value: Value,
        field: &'static str,
        allow_empty: bool,
    ) -> Result<Vec<String>, ProtocolError> {
        let Value::Array(values) = value else {
            return Err(Self::unexpected(field, "an array"));
        };
        if !allow_empty && values.is_empty() {
            return Err(ProtocolError::EmptyValue { field });
        }
        let values = values
            .into_iter()
            .map(|value| Self::text(value, field))
            .collect::<Result<Vec<_>, _>>()?;
        Self::require_unique(&values, field)?;
        Ok(values)
    }

    pub(crate) fn unsigned_array(
        value: Value,
        field: &'static str,
        allow_empty: bool,
    ) -> Result<Vec<u64>, ProtocolError> {
        let Value::Array(values) = value else {
            return Err(Self::unexpected(field, "an array"));
        };
        if !allow_empty && values.is_empty() {
            return Err(ProtocolError::EmptyValue { field });
        }
        let values = values
            .into_iter()
            .map(|value| Self::unsigned(value, field))
            .collect::<Result<Vec<_>, _>>()?;
        Self::require_unique(&values, field)?;
        Ok(values)
    }

    pub(crate) fn boolean_map(value: Value) -> Result<BTreeMap<String, bool>, ProtocolError> {
        let Value::Map(values) = value else {
            return Err(Self::unexpected("options", "a map"));
        };
        let mut result = BTreeMap::new();
        for (key, value) in values {
            let key = Self::text(key, "options key")?;
            if result
                .insert(key, Self::boolean(value, "options value")?)
                .is_some()
            {
                return Err(ProtocolError::DuplicateValue { field: "options" });
            }
        }
        Ok(result)
    }

    pub(crate) fn unsigned_map(value: Value) -> Result<BTreeMap<String, u64>, ProtocolError> {
        let Value::Map(values) = value else {
            return Err(Self::unexpected("certifications", "a map"));
        };
        let mut result = BTreeMap::new();
        for (key, value) in values {
            let key = Self::text(key, "certification identifier")?;
            if result
                .insert(key, Self::unsigned(value, "certification level")?)
                .is_some()
            {
                return Err(ProtocolError::DuplicateValue {
                    field: "certifications",
                });
            }
        }
        Ok(result)
    }

    pub(crate) fn text_map(
        value: Value,
        field: &'static str,
    ) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let Value::Map(values) = value else {
            return Err(Self::unexpected(field, "a map"));
        };
        let mut result = BTreeMap::new();
        for (key, value) in values {
            let key = Self::text(key, field)?;
            if result.insert(key, value).is_some() {
                return Err(ProtocolError::DuplicateValue { field });
            }
        }
        Ok(result)
    }

    pub(crate) fn unexpected(field: &'static str, expected: &'static str) -> ProtocolError {
        ProtocolError::UnexpectedType { field, expected }
    }

    fn require_unique<T>(values: &[T], field: &'static str) -> Result<(), ProtocolError>
    where
        T: Eq + std::hash::Hash,
    {
        let mut found = HashSet::with_capacity(values.len());
        if values.iter().all(|value| found.insert(value)) {
            Ok(())
        } else {
            Err(ProtocolError::DuplicateValue { field })
        }
    }
}
