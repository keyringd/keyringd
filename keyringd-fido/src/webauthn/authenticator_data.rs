use super::AuthenticationExtensionsAuthenticatorOutputs;
use super::CredentialPublicKey;
use super::{AttestedCredentialData, ProtocolError};
use crate::codec::CborCodec;
use sha2::{Digest, Sha256};

const PREFIX_LENGTH: usize = 37;
pub(crate) const FLAG_UP: u8 = 0x01;
pub(crate) const FLAG_RESERVED_1: u8 = 0x02;
pub(crate) const FLAG_UV: u8 = 0x04;
pub(crate) const FLAG_BE: u8 = 0x08;
pub(crate) const FLAG_BS: u8 = 0x10;
pub(crate) const FLAG_RESERVED_2: u8 = 0x20;
pub(crate) const FLAG_AT: u8 = 0x40;
pub(crate) const FLAG_ED: u8 = 0x80;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorData {
    rp_id_hash: [u8; 32],
    flags: u8,
    sign_count: u32,
    attested_credential_data: Option<AttestedCredentialData>,
    extensions: Option<AuthenticationExtensionsAuthenticatorOutputs>,
    raw: Vec<u8>,
}

impl AuthenticatorData {
    /// Builds assertion data without attested credential data or extensions.
    /// The caller supplies the authenticator's flags and signature counter.
    pub fn new(rp_id: &str, flags: u8, sign_count: u32) -> Result<Self, ProtocolError> {
        let mut data = Vec::with_capacity(PREFIX_LENGTH);
        data.extend_from_slice(&Sha256::digest(rp_id.as_bytes()));
        data.push(flags);
        data.extend_from_slice(&sign_count.to_be_bytes());
        Self::from_bytes(&data)
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() < PREFIX_LENGTH {
            return Err(ProtocolError::AuthenticatorDataTooShort);
        }
        let flags = data[32];
        let reserved = flags & (FLAG_RESERVED_1 | FLAG_RESERVED_2);
        if reserved != 0 {
            return Err(ProtocolError::ReservedAuthenticatorDataFlags(reserved));
        }
        if flags & FLAG_BS != 0 && flags & FLAG_BE == 0 {
            return Err(ProtocolError::InvalidBackupFlags);
        }
        let mut offset = PREFIX_LENGTH;
        let attested_credential_data = if flags & FLAG_AT != 0 {
            if data.len() < offset + 18 {
                return Err(ProtocolError::InvalidAttestedCredentialData);
            }
            let aaguid = data[offset..offset + 16]
                .try_into()
                .expect("sixteen-byte AAGUID slice");
            offset += 16;
            let credential_id_length =
                usize::from(u16::from_be_bytes([data[offset], data[offset + 1]]));
            offset += 2;
            if credential_id_length > 1023 {
                return Err(ProtocolError::CredentialIdTooLong {
                    actual: credential_id_length,
                });
            }
            if data.len() < offset + credential_id_length {
                return Err(ProtocolError::TruncatedCredentialId);
            }
            let credential_id = data[offset..offset + credential_id_length].to_vec();
            offset += credential_id_length;
            let key_length = CborCodec::encoded_item_length(&data[offset..])?;
            let credential_public_key =
                CredentialPublicKey::from_cbor_bytes(&data[offset..offset + key_length])?;
            offset += key_length;
            Some(AttestedCredentialData::new(
                aaguid,
                credential_id,
                credential_public_key,
            ))
        } else {
            None
        };
        let extensions = if flags & FLAG_ED != 0 {
            let length = CborCodec::encoded_item_length(&data[offset..])?;
            let encoded = AuthenticationExtensionsAuthenticatorOutputs::from_cbor_bytes(
                &data[offset..offset + length],
            )?;
            offset += length;
            Some(encoded)
        } else {
            None
        };
        if offset != data.len() {
            return Err(ProtocolError::TrailingAuthenticatorData);
        }
        Ok(Self {
            rp_id_hash: data[..32].try_into().expect("32-byte RP ID hash slice"),
            flags,
            sign_count: u32::from_be_bytes(data[33..37].try_into().expect("four-byte counter")),
            attested_credential_data,
            extensions,
            raw: data.to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut data = Vec::with_capacity(self.raw.len().max(PREFIX_LENGTH));
        data.extend_from_slice(&self.rp_id_hash);
        data.push(self.flags);
        data.extend_from_slice(&self.sign_count.to_be_bytes());
        match (&self.attested_credential_data, self.flags & FLAG_AT != 0) {
            (Some(attested), true) => {
                data.extend_from_slice(attested.aaguid());
                let length = u16::try_from(attested.credential_id().len())
                    .map_err(|_| ProtocolError::TruncatedCredentialId)?;
                data.extend_from_slice(&length.to_be_bytes());
                data.extend_from_slice(attested.credential_id());
                data.extend_from_slice(&attested.credential_public_key().to_cbor_bytes()?);
            }
            (None, false) => {}
            _ => return Err(ProtocolError::InvalidAttestedCredentialData),
        }
        match (&self.extensions, self.flags & FLAG_ED != 0) {
            (Some(extensions), true) => data.extend_from_slice(&extensions.to_cbor_bytes()?),
            (None, false) => {}
            _ => return Err(ProtocolError::InvalidAuthenticatorExtensions),
        }
        Ok(data)
    }

    pub const fn user_present(&self) -> bool {
        self.flags & FLAG_UP != 0
    }

    pub const fn rp_id_hash(&self) -> &[u8; 32] {
        &self.rp_id_hash
    }
    pub const fn flags(&self) -> u8 {
        self.flags
    }
    pub const fn sign_count(&self) -> u32 {
        self.sign_count
    }
    pub fn attested_credential_data(&self) -> Option<&AttestedCredentialData> {
        self.attested_credential_data.as_ref()
    }
    pub fn extensions(&self) -> Option<&AuthenticationExtensionsAuthenticatorOutputs> {
        self.extensions.as_ref()
    }
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    pub const fn user_verified(&self) -> bool {
        self.flags & FLAG_UV != 0
    }

    pub const fn backup_eligible(&self) -> bool {
        self.flags & FLAG_BE != 0
    }

    pub const fn backup_state(&self) -> bool {
        self.flags & FLAG_BS != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "AuthenticatorData::new" {
            case "Builds assertion data with the RP ID hash flags and supplied counter" {
                verify "the encoded assertion preserves its fields" {
                    let data = AuthenticatorData::new("ssh:keyringd", 0x05, 0x01020304).unwrap();
                    assert_eq!(data.rp_id_hash(), &[
                        0x91, 0x66, 0xa1, 0x16, 0x47, 0xb5, 0xd9, 0xf5,
                        0x35, 0xa5, 0x01, 0xad, 0x4d, 0x28, 0x7e, 0x9d,
                        0x85, 0xd0, 0xb3, 0x98, 0xde, 0x14, 0x95, 0x90,
                        0x2e, 0xcc, 0x68, 0x44, 0x54, 0x62, 0xa3, 0x05,
                    ]);
                    assert!(data.user_present());
                    assert!(data.user_verified());
                    assert_eq!(data.sign_count(), 0x01020304);
                    assert_eq!(&data.raw()[32..], &[5, 1, 2, 3, 4]);
                    assert!(data.attested_credential_data().is_none());
                    assert!(data.extensions().is_none());
                    assert_eq!(data.to_bytes().unwrap(), data.raw());
                    assert_eq!(AuthenticatorData::from_bytes(data.raw()).unwrap(), data);
                }
            }
            case "Rejects invalid flags and flags requiring absent assertion fields" {
                verify "construction enforces the authenticator data invariants" {
                    assert!(matches!(AuthenticatorData::new("ssh:keyringd", FLAG_RESERVED_1, 0), Err(ProtocolError::ReservedAuthenticatorDataFlags(_))));
                    assert!(matches!(AuthenticatorData::new("ssh:keyringd", FLAG_BS, 0), Err(ProtocolError::InvalidBackupFlags)));
                    assert!(AuthenticatorData::new("ssh:keyringd", FLAG_AT, 0).is_err());
                    assert!(AuthenticatorData::new("ssh:keyringd", FLAG_ED, 0).is_err());
                }
            }
        }
    }
}
