#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum UserVerificationRequirement {
    Required,
    #[default]
    Preferred,
    Discouraged,
    Unknown(String),
}

impl UserVerificationRequirement {
    pub fn new(value: impl Into<String>) -> Self {
        match value.into() {
            value if value == "required" => Self::Required,
            value if value == "preferred" => Self::Preferred,
            value if value == "discouraged" => Self::Discouraged,
            value => Self::Unknown(value),
        }
    }

    pub fn as_webauthn_str(&self) -> &str {
        match self {
            Self::Required => "required",
            Self::Preferred => "preferred",
            Self::Discouraged => "discouraged",
            Self::Unknown(value) => value,
        }
    }
}

impl serde::Serialize for UserVerificationRequirement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_webauthn_str())
    }
}

impl<'de> serde::Deserialize<'de> for UserVerificationRequirement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <String as serde::Deserialize>::deserialize(deserializer).map(Self::new)
    }
}
