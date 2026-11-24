use crate::codec::CborItem as Value;

use super::{CoseAlgorithmIdentifier, CoseEllipticCurve, CoseKeyOperation};
use crate::ProtocolError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ec2CredentialPublicKey {
    algorithm: CoseAlgorithmIdentifier,
    curve: CoseEllipticCurve,
    x: Vec<u8>,
    y: Vec<u8>,
    key_id: Option<Vec<u8>>,
    key_operations: Option<Vec<CoseKeyOperation>>,
    base_initialization_vector: Option<Vec<u8>>,
}

impl Ec2CredentialPublicKey {
    pub fn new(
        algorithm: CoseAlgorithmIdentifier,
        curve: CoseEllipticCurve,
        x: Vec<u8>,
        y: Vec<u8>,
    ) -> Self {
        Self {
            algorithm,
            curve,
            x,
            y,
            key_id: None,
            key_operations: None,
            base_initialization_vector: None,
        }
    }

    pub fn with_key_id(mut self, value: Vec<u8>) -> Self {
        self.key_id = Some(value);
        self
    }

    pub fn with_key_operations(mut self, value: Vec<CoseKeyOperation>) -> Self {
        self.key_operations = Some(value);
        self
    }

    pub fn with_base_initialization_vector(mut self, value: Vec<u8>) -> Self {
        self.base_initialization_vector = Some(value);
        self
    }

    pub const fn algorithm(&self) -> CoseAlgorithmIdentifier {
        self.algorithm
    }

    pub const fn curve(&self) -> CoseEllipticCurve {
        self.curve
    }

    pub fn x(&self) -> &[u8] {
        &self.x
    }

    pub fn y(&self) -> &[u8] {
        &self.y
    }

    pub fn key_id(&self) -> Option<&[u8]> {
        self.key_id.as_deref()
    }

    pub fn key_operations(&self) -> Option<&[CoseKeyOperation]> {
        self.key_operations.as_deref()
    }

    pub fn base_initialization_vector(&self) -> Option<&[u8]> {
        self.base_initialization_vector.as_deref()
    }

    pub(crate) fn from_cbor_fields(
        mut fields: std::collections::BTreeMap<i64, Value>,
    ) -> Result<Self, ProtocolError> {
        let algorithm = CoseAlgorithmIdentifier::new(
            i32::try_from(take_integer(&mut fields, 3, "COSE algorithm")?)
                .map_err(|_| unexpected("COSE algorithm", "a WebIDL long"))?,
        );
        let curve = CoseEllipticCurve::new(take_integer(&mut fields, -1, "EC2 curve")?);
        let x = take_bytes(&mut fields, -2, "EC2 x")?;
        let y = take_bytes(&mut fields, -3, "EC2 y")?;
        let key_id = take_optional_bytes(&mut fields, 2, "COSE key ID")?;
        let key_operations = fields
            .remove(&4)
            .map(|value| match value {
                Value::Array(values) => values
                    .into_iter()
                    .map(|value| integer(value, "COSE key operation").map(CoseKeyOperation::new))
                    .collect(),
                _ => Err(unexpected("COSE key operations", "an array")),
            })
            .transpose()?;
        let base_initialization_vector =
            take_optional_bytes(&mut fields, 5, "COSE base initialization vector")?;
        Ok(Self {
            algorithm,
            curve,
            x,
            y,
            key_id,
            key_operations,
            base_initialization_vector,
        })
    }

    pub(crate) fn to_cbor_entries(&self) -> Result<Vec<(Value, Value)>, ProtocolError> {
        let mut fields = vec![
            (Value::integer(1), Value::integer(2)),
            (
                Value::integer(3),
                Value::integer(i64::from(self.algorithm.value())),
            ),
            (Value::integer(-1), Value::integer(self.curve.value())),
            (Value::integer(-2), Value::ByteString(self.x.clone())),
            (Value::integer(-3), Value::ByteString(self.y.clone())),
        ];
        if let Some(value) = &self.key_id {
            fields.push((Value::integer(2), Value::ByteString(value.clone())));
        }
        if let Some(values) = &self.key_operations {
            fields.push((
                Value::integer(4),
                Value::Array(
                    values
                        .iter()
                        .map(|value| Value::integer(value.value()))
                        .collect(),
                ),
            ));
        }
        if let Some(value) = &self.base_initialization_vector {
            fields.push((Value::integer(5), Value::ByteString(value.clone())));
        }
        Ok(fields)
    }
}

fn take_integer(
    fields: &mut std::collections::BTreeMap<i64, Value>,
    label: i64,
    field: &'static str,
) -> Result<i64, ProtocolError> {
    integer(
        fields
            .remove(&label)
            .ok_or(ProtocolError::MissingField(field))?,
        field,
    )
}

fn take_bytes(
    fields: &mut std::collections::BTreeMap<i64, Value>,
    label: i64,
    field: &'static str,
) -> Result<Vec<u8>, ProtocolError> {
    match fields.remove(&label) {
        Some(Value::ByteString(value)) => Ok(value),
        Some(_) => Err(unexpected(field, "a byte string")),
        None => Err(ProtocolError::MissingField(field)),
    }
}

fn take_optional_bytes(
    fields: &mut std::collections::BTreeMap<i64, Value>,
    label: i64,
    field: &'static str,
) -> Result<Option<Vec<u8>>, ProtocolError> {
    fields
        .remove(&label)
        .map(|value| match value {
            Value::ByteString(value) => Ok(value),
            _ => Err(unexpected(field, "a byte string")),
        })
        .transpose()
}

fn integer(value: Value, field: &'static str) -> Result<i64, ProtocolError> {
    match value {
        Value::Unsigned(value) => {
            i64::try_from(value).map_err(|_| unexpected(field, "an integer representable as i64"))
        }
        Value::Negative(value) => {
            i64::try_from(value).map_err(|_| unexpected(field, "a signed 64-bit integer"))
        }
        _ => Err(unexpected(field, "an integer")),
    }
}

fn unexpected(field: &'static str, expected: &'static str) -> ProtocolError {
    ProtocolError::UnexpectedType { field, expected }
}
