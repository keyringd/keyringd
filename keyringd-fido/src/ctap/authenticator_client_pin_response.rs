use crate::codec::CborItem as Value;

use super::{CtapValue, PinUvKeyAgreementKey, ProtocolError};
use crate::codec::CborCodec;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorClientPinResponse {
    key_agreement: Option<PinUvKeyAgreementKey>,
    pin_uv_auth_token: Option<Vec<u8>>,
    pin_retries: Option<u64>,
    power_cycle_state: Option<bool>,
    uv_retries: Option<u64>,
}

impl AuthenticatorClientPinResponse {
    pub fn new() -> Self {
        Self {
            key_agreement: None,
            pin_uv_auth_token: None,
            pin_retries: None,
            power_cycle_state: None,
            uv_retries: None,
        }
    }
    pub fn with_key_agreement(mut self, value: PinUvKeyAgreementKey) -> Self {
        self.key_agreement = Some(value);
        self
    }
    pub fn with_pin_uv_auth_token(mut self, value: Vec<u8>) -> Self {
        self.pin_uv_auth_token = Some(value);
        self
    }
    pub fn with_pin_retries(mut self, value: u64) -> Self {
        self.pin_retries = Some(value);
        self
    }
    pub fn with_power_cycle_state(mut self, value: bool) -> Self {
        self.power_cycle_state = Some(value);
        self
    }
    pub fn with_uv_retries(mut self, value: u64) -> Self {
        self.uv_retries = Some(value);
        self
    }
    pub fn key_agreement(&self) -> Option<&PinUvKeyAgreementKey> {
        self.key_agreement.as_ref()
    }
    pub fn pin_uv_auth_token(&self) -> Option<&[u8]> {
        self.pin_uv_auth_token.as_deref()
    }
    pub const fn pin_retries(&self) -> Option<u64> {
        self.pin_retries
    }
    pub const fn power_cycle_state(&self) -> Option<bool> {
        self.power_cycle_state
    }
    pub const fn uv_retries(&self) -> Option<u64> {
        self.uv_retries
    }

    pub fn from_cbor_bytes(encoded: &[u8]) -> Result<Self, ProtocolError> {
        let mut fields = CtapValue::ctap_map(CborCodec::decode(encoded)?)?;
        Ok(Self {
            key_agreement: fields
                .remove(&1)
                .map(|value| PinUvKeyAgreementKey::from_cbor_bytes(&CborCodec::encode(value)?))
                .transpose()?,
            pin_uv_auth_token: fields
                .remove(&2)
                .map(|v| CtapValue::bytes(v, "pinUvAuthToken"))
                .transpose()?,
            pin_retries: fields
                .remove(&3)
                .map(|v| CtapValue::unsigned(v, "pinRetries"))
                .transpose()?,
            power_cycle_state: fields
                .remove(&4)
                .map(|v| CtapValue::boolean(v, "powerCycleState"))
                .transpose()?,
            uv_retries: fields
                .remove(&5)
                .map(|v| CtapValue::unsigned(v, "uvRetries"))
                .transpose()?,
        })
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut fields = Vec::new();
        if let Some(v) = &self.key_agreement {
            fields.push((1u64.into(), CborCodec::decode(&v.to_cbor_bytes()?)?));
        }
        if let Some(v) = &self.pin_uv_auth_token {
            fields.push((2u64.into(), Value::ByteString(v.clone())));
        }
        if let Some(v) = self.pin_retries {
            fields.push((3u64.into(), v.into()));
        }
        if let Some(v) = self.power_cycle_state {
            fields.push((4u64.into(), Value::bool_value(v)));
        }
        if let Some(v) = self.uv_retries {
            fields.push((5u64.into(), v.into()));
        }
        Ok(CborCodec::encode(Value::Map(fields))?)
    }
}

impl Default for AuthenticatorClientPinResponse {
    fn default() -> Self {
        Self::new()
    }
}
