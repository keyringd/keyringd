use crate::codec::CborItem as Value;

use super::packed_attestation_statement::{byte_string, byte_string_array, text_map};
use crate::ProtocolError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FidoU2fAttestationStatement {
    attestation_certificate: Vec<u8>,
    signature: Vec<u8>,
}

impl FidoU2fAttestationStatement {
    pub fn new(attestation_certificate: Vec<u8>, signature: Vec<u8>) -> Self {
        Self {
            attestation_certificate,
            signature,
        }
    }

    pub fn attestation_certificate(&self) -> &[u8] {
        &self.attestation_certificate
    }

    pub fn signature(&self) -> &[u8] {
        &self.signature
    }

    pub(crate) fn from_cbor_value(value: Value) -> Result<Self, ProtocolError> {
        let mut fields = text_map(value, "fido-u2f attestation statement")?;
        let certificates = byte_string_array(
            fields.remove("x5c").ok_or(ProtocolError::UnexpectedType {
                field: "fido-u2f.x5c",
                expected: "a required single-element certificate array",
            })?,
            "fido-u2f.x5c",
        )?;
        if certificates.len() != 1 {
            return Err(ProtocolError::InvalidLength {
                field: "fido-u2f.x5c",
                expected: 1,
                actual: certificates.len(),
            });
        }
        Ok(Self {
            attestation_certificate: certificates.into_iter().next().expect("one certificate"),
            signature: byte_string(
                fields.remove("sig").ok_or(ProtocolError::UnexpectedType {
                    field: "fido-u2f.sig",
                    expected: "a required byte string",
                })?,
                "fido-u2f.sig",
            )?,
        })
    }

    pub(crate) fn to_cbor_value(&self) -> Result<Value, ProtocolError> {
        let fields = vec![
            (
                Value::TextString("x5c".to_owned()),
                Value::Array(vec![Value::ByteString(
                    self.attestation_certificate.clone(),
                )]),
            ),
            (
                Value::TextString("sig".to_owned()),
                Value::ByteString(self.signature.clone()),
            ),
        ];
        Ok(Value::Map(fields))
    }
}
