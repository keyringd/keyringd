#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PublicKeyCredentialType {
    PublicKey,
    Unknown(String),
}

impl PublicKeyCredentialType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::PublicKey => "public-key",
            Self::Unknown(value) => value,
        }
    }
}

impl serde::Serialize for PublicKeyCredentialType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for PublicKeyCredentialType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self::from)
    }
}

impl From<String> for PublicKeyCredentialType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "public-key" => Self::PublicKey,
            _ => Self::Unknown(value),
        }
    }
}
