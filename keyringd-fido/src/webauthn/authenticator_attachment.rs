#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum AuthenticatorAttachment {
    Platform,
    CrossPlatform,
    Unknown(String),
}

impl AuthenticatorAttachment {
    pub fn new(value: impl Into<String>) -> Self {
        match value.into() {
            value if value == "platform" => Self::Platform,
            value if value == "cross-platform" => Self::CrossPlatform,
            value => Self::Unknown(value),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Platform => "platform",
            Self::CrossPlatform => "cross-platform",
            Self::Unknown(value) => value,
        }
    }
}

impl serde::Serialize for AuthenticatorAttachment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for AuthenticatorAttachment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <String as serde::Deserialize>::deserialize(deserializer).map(Self::new)
    }
}
