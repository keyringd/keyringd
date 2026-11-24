use crate::{
    ProtocolError,
    codec::{CborCodec, CborItem as Value, CborValue},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorLargeBlobsResponse {
    config: Vec<u8>,
}
impl AuthenticatorLargeBlobsResponse {
    pub fn new(config: Vec<u8>) -> Self {
        Self { config }
    }
    pub fn config(&self) -> &[u8] {
        &self.config
    }
    pub fn from_cbor_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut fields = CborValue::ctap_map(CborCodec::decode(bytes)?)?;
        Ok(Self::new(CborValue::bytes(
            fields
                .remove(&1)
                .ok_or(ProtocolError::MissingCtapField(1))?,
            "largeBlobs.config",
        )?))
    }
    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(CborCodec::encode(Value::Map(vec![(
            1u64.into(),
            Value::ByteString(self.config.clone()),
        )]))?)
    }
}
