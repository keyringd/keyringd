use crate::codec::CborItem as Value;

use super::{
    ClientPinSubcommand, CtapValue, PinUvAuthPermissions, PinUvKeyAgreementKey, ProtocolError,
};
use crate::codec::CborCodec;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorClientPinRequest {
    pub(crate) pin_uv_auth_protocol: Option<u64>,
    pub(crate) subcommand: ClientPinSubcommand,
    pub(crate) key_agreement: Option<PinUvKeyAgreementKey>,
    pub(crate) pin_uv_auth_param: Option<Vec<u8>>,
    pub(crate) new_pin_enc: Option<Vec<u8>>,
    pub(crate) pin_hash_enc: Option<Vec<u8>>,
    pub(crate) permissions: Option<PinUvAuthPermissions>,
    pub(crate) relying_party_id: Option<String>,
}

impl AuthenticatorClientPinRequest {
    pub fn new(subcommand: ClientPinSubcommand) -> Self {
        Self {
            pin_uv_auth_protocol: None,
            subcommand,
            key_agreement: None,
            pin_uv_auth_param: None,
            new_pin_enc: None,
            pin_hash_enc: None,
            permissions: None,
            relying_party_id: None,
        }
    }
    pub fn with_pin_uv_auth_protocol(mut self, value: u64) -> Self {
        self.pin_uv_auth_protocol = Some(value);
        self
    }
    pub fn with_key_agreement(mut self, value: PinUvKeyAgreementKey) -> Self {
        self.key_agreement = Some(value);
        self
    }
    pub fn with_pin_uv_auth_param(mut self, value: Vec<u8>) -> Self {
        self.pin_uv_auth_param = Some(value);
        self
    }
    pub fn with_new_pin_enc(mut self, value: Vec<u8>) -> Self {
        self.new_pin_enc = Some(value);
        self
    }
    pub fn with_pin_hash_enc(mut self, value: Vec<u8>) -> Self {
        self.pin_hash_enc = Some(value);
        self
    }
    pub fn with_permissions(mut self, value: PinUvAuthPermissions) -> Self {
        self.permissions = Some(value);
        self
    }
    pub fn with_relying_party_id(mut self, value: impl Into<String>) -> Self {
        self.relying_party_id = Some(value.into());
        self
    }
    pub const fn subcommand(&self) -> ClientPinSubcommand {
        self.subcommand
    }
    pub const fn pin_uv_auth_protocol(&self) -> Option<u64> {
        self.pin_uv_auth_protocol
    }
    pub fn key_agreement(&self) -> Option<&PinUvKeyAgreementKey> {
        self.key_agreement.as_ref()
    }
    pub fn pin_uv_auth_param(&self) -> Option<&[u8]> {
        self.pin_uv_auth_param.as_deref()
    }
    pub fn new_pin_enc(&self) -> Option<&[u8]> {
        self.new_pin_enc.as_deref()
    }
    pub fn pin_hash_enc(&self) -> Option<&[u8]> {
        self.pin_hash_enc.as_deref()
    }
    pub const fn permissions(&self) -> Option<PinUvAuthPermissions> {
        self.permissions
    }
    pub fn relying_party_id(&self) -> Option<&str> {
        self.relying_party_id.as_deref()
    }

    pub fn from_cbor_bytes(encoded: &[u8]) -> Result<Self, ProtocolError> {
        let mut fields = CtapValue::ctap_map(CborCodec::decode(encoded)?)?;
        let request = Self {
            pin_uv_auth_protocol: fields
                .remove(&1)
                .map(|v| CtapValue::unsigned(v, "pinUvAuthProtocol"))
                .transpose()?,
            subcommand: ClientPinSubcommand::from_u64(CtapValue::unsigned(
                fields
                    .remove(&2)
                    .ok_or(ProtocolError::MissingCtapField(2))?,
                "subCommand",
            )?),
            key_agreement: fields
                .remove(&3)
                .map(|value| PinUvKeyAgreementKey::from_cbor_bytes(&CborCodec::encode(value)?))
                .transpose()?,
            pin_uv_auth_param: fields
                .remove(&4)
                .map(|v| CtapValue::bytes(v, "pinUvAuthParam"))
                .transpose()?,
            new_pin_enc: fields
                .remove(&5)
                .map(|v| CtapValue::bytes(v, "newPinEnc"))
                .transpose()?,
            pin_hash_enc: fields
                .remove(&6)
                .map(|v| CtapValue::bytes(v, "pinHashEnc"))
                .transpose()?,
            permissions: fields
                .remove(&9)
                .map(|v| {
                    CtapValue::unsigned(v, "permissions")
                        .map(PinUvAuthPermissions::from_bits_retain)
                })
                .transpose()?,
            relying_party_id: fields
                .remove(&10)
                .map(|v| CtapValue::text(v, "rpId"))
                .transpose()?,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;
        let mut fields = vec![(2u64.into(), self.subcommand.as_u64().into())];
        if let Some(v) = self.pin_uv_auth_protocol {
            fields.push((1u64.into(), v.into()));
        }
        if let Some(v) = &self.key_agreement {
            fields.push((3u64.into(), CborCodec::decode(&v.to_cbor_bytes()?)?));
        }
        if let Some(v) = &self.pin_uv_auth_param {
            fields.push((4u64.into(), Value::ByteString(v.clone())));
        }
        if let Some(v) = &self.new_pin_enc {
            fields.push((5u64.into(), Value::ByteString(v.clone())));
        }
        if let Some(v) = &self.pin_hash_enc {
            fields.push((6u64.into(), Value::ByteString(v.clone())));
        }
        if let Some(v) = self.permissions {
            fields.push((9u64.into(), v.bits().into()));
        }
        if let Some(v) = &self.relying_party_id {
            fields.push((10u64.into(), Value::TextString(v.clone())));
        }
        Ok(CborCodec::encode(Value::Map(fields))?)
    }

    fn validate(&self) -> Result<(), ProtocolError> {
        if self.permissions.is_some_and(|v| v.bits() == 0) {
            return Err(ProtocolError::MustBePositive {
                field: "permissions",
            });
        }

        match self.subcommand {
            ClientPinSubcommand::GetPinRetries | ClientPinSubcommand::GetUvRetries => {}
            ClientPinSubcommand::GetKeyAgreement => {
                self.require_protocol()?;
            }
            ClientPinSubcommand::SetPin => {
                self.require_protocol()?;
                require(self.key_agreement.as_ref(), "keyAgreement")?;
                require(self.pin_uv_auth_param.as_ref(), "pinUvAuthParam")?;
                require(self.new_pin_enc.as_ref(), "newPinEnc")?;
            }
            ClientPinSubcommand::ChangePin => {
                self.require_protocol()?;
                require(self.key_agreement.as_ref(), "keyAgreement")?;
                require(self.pin_uv_auth_param.as_ref(), "pinUvAuthParam")?;
                require(self.new_pin_enc.as_ref(), "newPinEnc")?;
                require(self.pin_hash_enc.as_ref(), "pinHashEnc")?;
            }
            ClientPinSubcommand::GetPinToken => {
                self.require_protocol()?;
                require(self.key_agreement.as_ref(), "keyAgreement")?;
                require(self.pin_hash_enc.as_ref(), "pinHashEnc")?;
                forbid(
                    self.permissions.as_ref(),
                    "permissions",
                    "absent for getPinToken",
                )?;
                forbid(
                    self.relying_party_id.as_ref(),
                    "rpId",
                    "absent for getPinToken",
                )?;
            }
            ClientPinSubcommand::GetPinUvAuthTokenUsingUvWithPermissions => {
                self.require_protocol()?;
                require(self.key_agreement.as_ref(), "keyAgreement")?;
                self.require_permissions()?;
            }
            ClientPinSubcommand::GetPinUvAuthTokenUsingPinWithPermissions => {
                self.require_protocol()?;
                require(self.key_agreement.as_ref(), "keyAgreement")?;
                require(self.pin_hash_enc.as_ref(), "pinHashEnc")?;
                self.require_permissions()?;
            }
            ClientPinSubcommand::Unknown(_) => {}
        }
        Ok(())
    }

    fn require_protocol(&self) -> Result<(), ProtocolError> {
        require(self.pin_uv_auth_protocol.as_ref(), "pinUvAuthProtocol").map(|_| ())
    }

    fn require_permissions(&self) -> Result<(), ProtocolError> {
        let permissions = require(self.permissions.as_ref(), "permissions")?;
        if (permissions.contains(PinUvAuthPermissions::MAKE_CREDENTIAL)
            || permissions.contains(PinUvAuthPermissions::GET_ASSERTION))
            && self.relying_party_id.is_none()
        {
            return Err(ProtocolError::MissingField("rpId"));
        }
        Ok(())
    }
}

fn require<'a, T>(value: Option<&'a T>, field: &'static str) -> Result<&'a T, ProtocolError> {
    value.ok_or(ProtocolError::MissingField(field))
}

fn forbid<T>(
    value: Option<&T>,
    field: &'static str,
    expected: &'static str,
) -> Result<(), ProtocolError> {
    if value.is_some() {
        return Err(ProtocolError::UnexpectedType { field, expected });
    }
    Ok(())
}
