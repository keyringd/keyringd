#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttestationConveyancePreference {
    None,
    Indirect,
    Direct,
    Enterprise,
    Unknown(String),
}

impl AttestationConveyancePreference {
    pub fn new(value: impl Into<String>) -> Self {
        match value.into() {
            value if value == "none" => Self::None,
            value if value == "indirect" => Self::Indirect,
            value if value == "direct" => Self::Direct,
            value if value == "enterprise" => Self::Enterprise,
            value => Self::Unknown(value),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::None => "none",
            Self::Indirect => "indirect",
            Self::Direct => "direct",
            Self::Enterprise => "enterprise",
            Self::Unknown(value) => value,
        }
    }
}

impl serde::Serialize for AttestationConveyancePreference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for AttestationConveyancePreference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <String as serde::Deserialize>::deserialize(deserializer).map(Self::new)
    }
}
