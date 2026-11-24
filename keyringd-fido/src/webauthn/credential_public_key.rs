use std::collections::BTreeMap;

use spki::der::{Encode, asn1::BitStringRef};
use spki::{AlgorithmIdentifier, ObjectIdentifier, SubjectPublicKeyInfo};

use crate::codec::CborItem as Value;

use super::{
    CoseAlgorithmIdentifier, CoseEllipticCurve, Ec2CredentialPublicKey, UnknownCredentialPublicKey,
};
use crate::codec::CborCodec;
use crate::{EncodedCborValue, ProtocolError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CredentialPublicKey {
    Ec2(Ec2CredentialPublicKey),
    Unknown(UnknownCredentialPublicKey),
}

impl CredentialPublicKey {
    pub fn from_cbor_bytes(encoded: &[u8]) -> Result<Self, ProtocolError> {
        let value = CborCodec::decode(encoded)?;
        let Value::Map(entries) = &value else {
            return Err(unexpected("credentialPublicKey", "a COSE_Key map"));
        };
        let mut fields = BTreeMap::new();
        for (key, value) in entries.iter().cloned() {
            let key = integer(key, "COSE key label")?;
            if fields.insert(key, value).is_some() {
                return Err(ProtocolError::DuplicateValue {
                    field: "COSE key label",
                });
            }
        }
        let key_type = integer(
            fields
                .get(&1)
                .cloned()
                .ok_or(ProtocolError::MissingField("COSE key type"))?,
            "COSE key type",
        )?;
        let algorithm = CoseAlgorithmIdentifier::new(
            i32::try_from(integer(
                fields
                    .get(&3)
                    .cloned()
                    .ok_or(ProtocolError::MissingField("COSE algorithm"))?,
                "COSE algorithm",
            )?)
            .map_err(|_| unexpected("COSE algorithm", "a WebIDL long"))?,
        );
        if key_type == 2 {
            fields.remove(&1);
            Ok(Self::Ec2(Ec2CredentialPublicKey::from_cbor_fields(fields)?))
        } else {
            Ok(Self::Unknown(UnknownCredentialPublicKey::new(
                key_type,
                algorithm,
                EncodedCborValue::from_value(value)?,
            )))
        }
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        match self {
            Self::Ec2(key) => Ok(CborCodec::encode(Value::Map(key.to_cbor_entries()?))?),
            Self::Unknown(key) => Ok(key.encoded().as_bytes().to_vec()),
        }
    }

    pub const fn algorithm(&self) -> CoseAlgorithmIdentifier {
        match self {
            Self::Ec2(key) => key.algorithm(),
            Self::Unknown(key) => key.algorithm(),
        }
    }

    pub(crate) fn to_spki_der(&self) -> Result<Option<Vec<u8>>, ProtocolError> {
        let Self::Ec2(key) = self else {
            return Ok(None);
        };
        if key.algorithm() != CoseAlgorithmIdentifier::ES256
            || key.curve() != CoseEllipticCurve::P256
        {
            return Ok(None);
        }
        let point = self.es256_sec1_public_key()?;
        let spki = SubjectPublicKeyInfo {
            algorithm: AlgorithmIdentifier {
                // id-ecPublicKey with the named curve prime256v1 (P-256).
                oid: ObjectIdentifier::new_unwrap("1.2.840.10045.2.1"),
                parameters: Some(ObjectIdentifier::new_unwrap("1.2.840.10045.3.1.7")),
            },
            subject_public_key: BitStringRef::from_bytes(&point)?,
        };
        Ok(Some(spki.to_der()?))
    }

    pub fn es256_sec1_public_key(&self) -> Result<[u8; 65], ProtocolError> {
        let Self::Ec2(key) = self else {
            return Err(unexpected("credentialPublicKey", "an ES256 EC2 key"));
        };
        if key.algorithm() != CoseAlgorithmIdentifier::ES256
            || key.curve() != CoseEllipticCurve::P256
        {
            return Err(unexpected("credentialPublicKey", "an ES256 EC2 key"));
        }
        if key.x().len() != 32 || key.y().len() != 32 {
            return Err(ProtocolError::InvalidLength {
                field: "EC2 coordinate",
                expected: 32,
                actual: key.x().len().min(key.y().len()),
            });
        }
        let mut result = [0u8; 65];
        result[0] = 4;
        result[1..33].copy_from_slice(key.x());
        result[33..].copy_from_slice(key.y());
        Ok(result)
    }

    pub fn from_es256_sec1_public_key(key: &[u8; 65]) -> Result<Self, ProtocolError> {
        if key[0] != 0x04 {
            return Err(unexpected(
                "credentialPublicKey",
                "an uncompressed P-256 SEC1 public key",
            ));
        }
        Ok(Self::Ec2(Ec2CredentialPublicKey::new(
            CoseAlgorithmIdentifier::ES256,
            CoseEllipticCurve::P256,
            key[1..33].to_vec(),
            key[33..].to_vec(),
        )))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "CredentialPublicKey::from_cbor_bytes" {
            case "Preserve unknown algorithms with typed EC2 keys" {
                why "A structurally valid EC2 COSE key can use an algorithm added after this crate; retaining its identifier keeps decoding lossless without implying verification support."
                spec "https://www.w3.org/TR/webauthn-3/#sctn-attested-credential-data"
                spec "https://www.rfc-editor.org/rfc/rfc9052.html#section-7"
                given "Prepare an EC2 Credential Public Key with an unknown Algorithm identifier" {
                    let key = CredentialPublicKey::Ec2(Ec2CredentialPublicKey::new(
                        CoseAlgorithmIdentifier::new(-999),
                        CoseEllipticCurve::P256,
                        vec![0x11; 32],
                        vec![0x22; 32],
                    ));
                }
                when "Encode the key to CBOR and then decode" {
                    let decoded = CredentialPublicKey::from_cbor_bytes(&key.to_cbor_bytes().unwrap()).unwrap();
                }
                then "The original typed key including unknown Algorithm is restored" {
                    assert_eq!(decoded, key);
                }
            }
            case "Keep unknown key types as regular CBORs" {
                why "Unknown COSE key types must remain available as canonical bytes so a newer key representation does not make the surrounding WebAuthn object undecodable."
                spec "https://www.w3.org/TR/webauthn-3/#sctn-attested-credential-data"
                spec "https://www.rfc-editor.org/rfc/rfc9052.html#section-7"
                given "Prepare CBORCODEC, VALUE" {
                    let encoded = CborCodec::encode(Value::Map(vec![
                        (Value::integer(1), Value::integer(999)),
                        (Value::integer(3), Value::integer(-999)),
                        (Value::integer(-1), Value::TextString("future".to_owned())),
                    ]))
                    .unwrap();
                }
                when "Call CredentialPublicKey::from_cbor_bytes" {
                    let key = CredentialPublicKey::from_cbor_bytes(&encoded).unwrap();
                }
                then "Keep unknown key types as regular CBORs" {
                    assert!(matches!(&key, CredentialPublicKey::Unknown(value) if value.key_type() == 999));
                    assert_eq!(key.to_cbor_bytes().unwrap(), encoded);
                }
            }
        }
    }
}
