use crate::codec::CborItem as Value;

use super::codec::{bytes, decode_canonical_map, encode_map};
use crate::ProtocolError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridPostHandshakeMessage {
    pub(crate) padding: Option<Vec<u8>>,
    pub(crate) get_info_response: Vec<u8>,
    pub(crate) supported_features: Vec<String>,
}

impl HybridPostHandshakeMessage {
    pub fn new(get_info_response: Vec<u8>) -> Self {
        Self {
            padding: None,
            get_info_response,
            supported_features: vec!["ctap".to_owned()],
        }
    }
    pub fn with_supported_features(mut self, value: Vec<String>) -> Self {
        self.supported_features = value;
        self
    }
    pub fn get_info_response(&self) -> &[u8] {
        &self.get_info_response
    }
    pub fn supported_features(&self) -> &[String] {
        &self.supported_features
    }
    pub fn from_cbor_bytes(encoded: &[u8]) -> Result<Self, ProtocolError> {
        let mut fields = decode_canonical_map(encoded, "hybrid post-handshake message")?;
        let padding = fields
            .remove(&0)
            .map(|value| bytes(value, "hybrid post-handshake padding"))
            .transpose()?;
        if padding
            .as_ref()
            .is_some_and(|bytes| bytes.iter().any(|byte| *byte != 0))
        {
            return Err(ProtocolError::UnexpectedType {
                field: "hybrid post-handshake padding",
                expected: "a byte string containing only zero bytes",
            });
        }
        let get_info_response = bytes(
            fields
                .remove(&1)
                .ok_or(ProtocolError::MissingCtapField(1))?,
            "hybrid post-handshake getInfo response",
        )?;
        let supported_features = match fields.remove(&3) {
            None => vec!["ctap".to_owned()],
            Some(Value::Array(values)) => values
                .into_iter()
                .map(|value| match value {
                    Value::TextString(value) => Ok(value),
                    _ => Err(ProtocolError::UnexpectedType {
                        field: "hybrid post-handshake supported features",
                        expected: "an array of text strings",
                    }),
                })
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => {
                return Err(ProtocolError::UnexpectedType {
                    field: "hybrid post-handshake supported features",
                    expected: "an array of text strings",
                });
            }
        };
        Ok(Self {
            padding,
            get_info_response,
            supported_features,
        })
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        if self
            .padding
            .as_ref()
            .is_some_and(|bytes| bytes.iter().any(|byte| *byte != 0))
        {
            return Err(ProtocolError::UnexpectedType {
                field: "hybrid post-handshake padding",
                expected: "a byte string containing only zero bytes",
            });
        }
        let mut fields = vec![(1, Value::ByteString(self.get_info_response.clone()))];
        if let Some(padding) = &self.padding {
            fields.push((0, Value::ByteString(padding.clone())));
        }
        if self.supported_features != ["ctap"] {
            fields.push((
                3,
                Value::Array(
                    self.supported_features
                        .iter()
                        .cloned()
                        .map(Value::TextString)
                        .collect(),
                ),
            ));
        }
        encode_map(fields)
    }
}
