/// WebAuthn LargeBlobSupport values. Unknown DOMString values are preserved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LargeBlobSupport {
    Required,
    Preferred,
    Unknown(String),
}

impl LargeBlobSupport {
    pub fn new(value: impl Into<String>) -> Self {
        match value.into() {
            value if value == "required" => Self::Required,
            value if value == "preferred" => Self::Preferred,
            value => Self::Unknown(value),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Required => "required",
            Self::Preferred => "preferred",
            Self::Unknown(value) => value,
        }
    }
}

impl serde::Serialize for LargeBlobSupport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for LargeBlobSupport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <String as serde::Deserialize>::deserialize(deserializer).map(Self::new)
    }
}
