use std::collections::BTreeMap;

use crate::codec::CborItem as Value;

use super::{AttestationStatement, AuthenticatorData, ProtocolError};
use crate::codec::CborCodec;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationObject {
    pub(crate) authenticator_data: AuthenticatorData,
    pub(crate) attestation_statement: AttestationStatement,
}

impl AttestationObject {
    pub fn new(
        authenticator_data: AuthenticatorData,
        attestation_statement: AttestationStatement,
    ) -> Self {
        Self {
            authenticator_data,
            attestation_statement,
        }
    }

    pub fn format(&self) -> &str {
        self.attestation_statement.format()
    }

    pub const fn authenticator_data(&self) -> &AuthenticatorData {
        &self.authenticator_data
    }

    pub const fn attestation_statement(&self) -> &AttestationStatement {
        &self.attestation_statement
    }

    pub fn from_cbor_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let Value::Map(entries) = CborCodec::decode(bytes)? else {
            return Err(ProtocolError::AttestationObjectNotAMap);
        };
        let mut fields = BTreeMap::new();
        for (key, value) in entries {
            let Value::TextString(key) = key else {
                return Err(ProtocolError::UnexpectedType {
                    field: "map key",
                    expected: "a text string",
                });
            };
            if fields.insert(key.clone(), value).is_some() {
                return Err(ProtocolError::DuplicateField(key));
            }
        }
        let format = match fields
            .remove("fmt")
            .ok_or(ProtocolError::MissingField("fmt"))?
        {
            Value::TextString(value) => value,
            _ => {
                return Err(ProtocolError::UnexpectedType {
                    field: "fmt",
                    expected: "a text string",
                });
            }
        };
        let authenticator_data = match fields
            .remove("authData")
            .ok_or(ProtocolError::MissingField("authData"))?
        {
            Value::ByteString(value) => AuthenticatorData::from_bytes(&value)?,
            _ => {
                return Err(ProtocolError::UnexpectedType {
                    field: "authData",
                    expected: "a byte string",
                });
            }
        };
        let attestation_statement = fields
            .remove("attStmt")
            .ok_or(ProtocolError::MissingField("attStmt"))?;
        if !matches!(attestation_statement, Value::Map(_)) {
            return Err(ProtocolError::UnexpectedType {
                field: "attStmt",
                expected: "a map",
            });
        }
        let attestation_statement =
            AttestationStatement::from_cbor_value(&format, attestation_statement)?;
        Ok(Self {
            authenticator_data,
            attestation_statement,
        })
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        let statement = self.attestation_statement.to_cbor_value()?;
        let entries = vec![
            (
                Value::TextString("fmt".to_owned()),
                Value::TextString(self.format().to_owned()),
            ),
            (
                Value::TextString("authData".to_owned()),
                Value::ByteString(self.authenticator_data.to_bytes()?),
            ),
            (Value::TextString("attStmt".to_owned()), statement),
        ];
        Ok(CborCodec::encode(Value::Map(entries))?)
    }
}
