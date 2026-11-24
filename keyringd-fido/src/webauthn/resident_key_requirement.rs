#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResidentKeyRequirement {
    Discouraged,
    Preferred,
    Required,
    Unknown(String),
}

impl ResidentKeyRequirement {
    pub fn new(value: impl Into<String>) -> Self {
        match value.into() {
            value if value == "discouraged" => Self::Discouraged,
            value if value == "preferred" => Self::Preferred,
            value if value == "required" => Self::Required,
            value => Self::Unknown(value),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Discouraged => "discouraged",
            Self::Preferred => "preferred",
            Self::Required => "required",
            Self::Unknown(value) => value,
        }
    }
}

impl serde::Serialize for ResidentKeyRequirement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ResidentKeyRequirement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <String as serde::Deserialize>::deserialize(deserializer).map(Self::new)
    }
}
