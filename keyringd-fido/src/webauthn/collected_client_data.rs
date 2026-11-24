use std::collections::BTreeMap;
use std::fmt;

use base64::Engine;

use super::{Origin, ProtocolError};
use crate::codec::JsonCodec;

#[derive(Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectedClientData {
    #[serde(rename = "type")]
    ceremony_type: String,
    challenge: String,
    origin: Origin,
    #[serde(default, serialize_with = "serialize_cross_origin")]
    cross_origin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_origin: Option<Origin>,
    #[serde(flatten)]
    additional_members: BTreeMap<String, serde_json::Value>,
}

impl CollectedClientData {
    pub fn create(challenge: &[u8], origin: &str) -> Result<Self, ProtocolError> {
        Self::generate("webauthn.create", challenge, origin)
    }

    pub fn get(challenge: &[u8], origin: &str) -> Result<Self, ProtocolError> {
        Self::generate("webauthn.get", challenge, origin)
    }

    pub fn from_json_bytes(json: &[u8]) -> Result<Self, ProtocolError> {
        let value: Self = JsonCodec::decode(json)?;
        validate_challenge(&value.challenge)?;
        Ok(value)
    }

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(JsonCodec::encode(self)?)
    }
    pub fn ceremony_type(&self) -> &str {
        &self.ceremony_type
    }
    pub const fn origin(&self) -> &Origin {
        &self.origin
    }
    pub const fn cross_origin(&self) -> Option<bool> {
        self.cross_origin
    }
    pub fn top_origin(&self) -> Option<&Origin> {
        self.top_origin.as_ref()
    }
    pub fn challenge_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&self.challenge)
            .map_err(|error| ProtocolError::InvalidChallenge(Some(error)))
    }

    fn generate(
        ceremony_type: &str,
        challenge: &[u8],
        origin: &str,
    ) -> Result<Self, ProtocolError> {
        Ok(Self {
            ceremony_type: ceremony_type.to_owned(),
            challenge: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(challenge),
            origin: Origin::parse(origin)?,
            cross_origin: None,
            top_origin: None,
            additional_members: BTreeMap::new(),
        })
    }
}

fn serialize_cross_origin<S>(value: &Option<bool>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_bool(value.unwrap_or(false))
}

impl fmt::Debug for CollectedClientData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CollectedClientData")
            .field("ceremony_type", &self.ceremony_type)
            .field("origin", &self.origin)
            .field("cross_origin", &self.cross_origin)
            .field("top_origin", &self.top_origin)
            .finish_non_exhaustive()
    }
}

fn validate_challenge(challenge: &str) -> Result<(), ProtocolError> {
    if challenge.contains('=') {
        return Err(ProtocolError::InvalidChallenge(None));
    }
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(challenge)
        .map_err(|error| ProtocolError::InvalidChallenge(Some(error)))?;
    if base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(decoded) != challenge {
        return Err(ProtocolError::InvalidChallenge(None));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "CollectedClientData::from_json_bytes" {
            case "Ignore unknown fields" {
                why "Collected client data is an extensible JSON object, so unknown members must not break parsing by older library versions."
                spec "https://www.w3.org/TR/webauthn-3/#dictionary-client-data"
                given "Prepare a Collected Client Data JSON with an unknown Future field" {
                    let json = br#"{ "type":"webauthn.get", "challenge":"AQ", "origin":"https://example.com", "future": [1, 2] }"#;
                }
                when "Decode to semantic type and re-encode to JSON" {
                    let data = CollectedClientData::from_json_bytes(json).unwrap();
                    let encoded = data.to_json_bytes().unwrap();
                }
                then "Analyze known FIELDs and retain unknown fields without losing them" {
                    assert_eq!(data.ceremony_type(), "webauthn.get");
                    assert_eq!(
                        serde_json::from_slice::<serde_json::Value>(&encoded).unwrap()["future"],
                        serde_json::json!([1, 2])
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CollectedClientData::from_json_bytes" {
            case "Rejects Duplicate Fields and Non-Normal Challenge" {
                why "Rejecting duplicate security-sensitive members and non-canonical challenge text prevents parser differentials and ambiguous signed client data."
                spec "https://www.w3.org/TR/webauthn-3/#dictionary-client-data"
                given "Prepare a JSON with Duplicate Fields and a JSON with Padding Challenge" {
                    let duplicate = br#"{"type":"webauthn.get","type":"future","challenge":"AQ","origin":"https://example.com"}"#;
                    let padded = br#"{"type":"webauthn.get","challenge":"AQ==","origin":"https://example.com"}"#;
                }
                when "Decode both as Collected Client Data" {
                    let duplicate_result = CollectedClientData::from_json_bytes(duplicate);
                    let padded_result = CollectedClientData::from_json_bytes(padded);
                }
                then "Rejected as Duplicate Field and Non-Canonical Challenge respectively" {
                    assert!(matches!(
                        duplicate_result,
                        Err(ProtocolError::Codec(crate::CodecError::InvalidJson(_)))
                    ));
                    assert!(matches!(
                        padded_result,
                        Err(ProtocolError::InvalidChallenge(None))
                    ));
                }
            }
        }
    }
}
