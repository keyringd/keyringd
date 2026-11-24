use std::collections::HashSet;

use crate::codec::CborItem as Value;

use super::{CoseAlgorithmIdentifier, PublicKeyCredentialType};
use crate::ProtocolError;
use crate::codec::CborValue;

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PublicKeyCredentialParameters {
    #[serde(rename = "type")]
    credential_type: PublicKeyCredentialType,
    #[serde(rename = "alg")]
    algorithm: CoseAlgorithmIdentifier,
}

impl PublicKeyCredentialParameters {
    pub fn es256() -> Self {
        Self::new(
            PublicKeyCredentialType::PublicKey,
            CoseAlgorithmIdentifier::ES256,
        )
    }
    pub fn new(
        credential_type: PublicKeyCredentialType,
        algorithm: CoseAlgorithmIdentifier,
    ) -> Self {
        Self {
            credential_type,
            algorithm,
        }
    }

    pub fn credential_type(&self) -> &PublicKeyCredentialType {
        &self.credential_type
    }
    pub const fn algorithm(&self) -> CoseAlgorithmIdentifier {
        self.algorithm
    }

    pub(crate) fn from_cbor_value(value: Value) -> Result<Self, ProtocolError> {
        let mut fields = CborValue::text_map(value, "public key credential parameters")?;
        let credential_type = CborValue::text(
            fields
                .remove("type")
                .ok_or(ProtocolError::MissingField("type"))?,
            "type",
        )?;
        let algorithm =
            match fields
                .remove("alg")
                .ok_or(ProtocolError::MissingField("alg"))?
            {
                Value::Unsigned(value) => i32::try_from(value)
                    .map_err(|_| CborValue::unexpected("alg", "a WebIDL long"))?,
                Value::Negative(value) => i32::try_from(value)
                    .map_err(|_| CborValue::unexpected("alg", "a WebIDL long"))?,
                _ => return Err(CborValue::unexpected("alg", "a WebIDL long")),
            };
        let credential_type = PublicKeyCredentialType::from(credential_type);
        Ok(Self {
            credential_type,
            algorithm: CoseAlgorithmIdentifier::new(algorithm),
        })
    }

    pub(crate) fn to_cbor_value(&self) -> Result<Value, ProtocolError> {
        let fields = vec![
            (
                Value::TextString("type".to_owned()),
                Value::TextString(self.credential_type.as_str().to_owned()),
            ),
            (
                Value::TextString("alg".to_owned()),
                Value::integer(i64::from(self.algorithm.value())),
            ),
        ];
        Ok(Value::Map(fields))
    }

    pub(crate) fn vec_from_value(value: Value) -> Result<Vec<Self>, ProtocolError> {
        let Value::Array(values) = value else {
            return Err(CborValue::unexpected("pubKeyCredParams", "an array"));
        };
        if values.is_empty() {
            return Err(ProtocolError::EmptyValue {
                field: "pubKeyCredParams",
            });
        }
        let result = values
            .into_iter()
            .map(Self::from_cbor_value)
            .collect::<Result<Vec<_>, _>>()?;
        let mut unique = HashSet::with_capacity(result.len());
        if !result
            .iter()
            .all(|value| unique.insert((value.credential_type.as_str(), value.algorithm)))
        {
            return Err(ProtocolError::DuplicateValue {
                field: "pubKeyCredParams",
            });
        }
        Ok(result)
    }

    pub(crate) fn vec_to_value(values: &[Self]) -> Result<Value, ProtocolError> {
        Ok(Value::Array(
            values
                .iter()
                .map(Self::to_cbor_value)
                .collect::<Result<_, _>>()?,
        ))
    }
}
