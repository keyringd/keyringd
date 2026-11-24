/// A WebAuthn PublicKeyCredentialHint.
///
/// This is a DOMString typedef, not a closed WebIDL enum, so future values
/// remain representable.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PublicKeyCredentialHint(String);

impl PublicKeyCredentialHint {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn security_key() -> Self {
        Self::new("security-key")
    }

    pub fn client_device() -> Self {
        Self::new("client-device")
    }

    pub fn hybrid() -> Self {
        Self::new("hybrid")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl serde::Serialize for PublicKeyCredentialHint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for PublicKeyCredentialHint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <String as serde::Deserialize>::deserialize(deserializer).map(Self::new)
    }
}
