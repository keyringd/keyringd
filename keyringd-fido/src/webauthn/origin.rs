use std::fmt;
use std::str::FromStr;

use http::Uri;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::ProtocolError;

/// A validated serialized origin as defined by RFC 6454.
///
/// The URI representation is intentionally private so that it does not become
/// part of keyringd-fido's public API.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Origin(Uri);

impl Origin {
    pub fn parse(value: &str) -> Result<Self, ProtocolError> {
        let uri = value
            .parse::<Uri>()
            .map_err(|error| ProtocolError::InvalidOrigin(Some(error)))?;
        let scheme = uri.scheme().ok_or(ProtocolError::InvalidOrigin(None))?;
        let authority = uri.authority().ok_or(ProtocolError::InvalidOrigin(None))?;
        let serialized = format!("{scheme}://{authority}");

        if authority.host().is_empty() || uri.query().is_some() || value != serialized {
            return Err(ProtocolError::InvalidOrigin(None));
        }

        Ok(Self(uri))
    }
}

impl FromStr for Origin {
    type Err = ProtocolError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for Origin {
    type Error = ProtocolError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}://{}",
            self.0.scheme().expect("validated origin has a scheme"),
            self.0
                .authority()
                .expect("validated origin has an authority")
        )
    }
}

impl Serialize for Origin {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Origin {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}
