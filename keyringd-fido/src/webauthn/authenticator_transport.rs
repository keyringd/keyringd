/// FIDO authenticator transport identifier (WebAuthn spec §5.8.4).
///
/// Unknown values are retained for forward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AuthenticatorTransport(String);

impl AuthenticatorTransport {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn usb() -> Self {
        Self::new("usb")
    }

    pub fn nfc() -> Self {
        Self::new("nfc")
    }

    pub fn ble() -> Self {
        Self::new("ble")
    }

    pub fn smart_card() -> Self {
        Self::new("smart-card")
    }

    pub fn internal() -> Self {
        Self::new("internal")
    }

    pub fn hybrid() -> Self {
        Self::new("hybrid")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl serde::Serialize for AuthenticatorTransport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for AuthenticatorTransport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <String as serde::Deserialize>::deserialize(deserializer).map(Self::new)
    }
}

impl From<String> for AuthenticatorTransport {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for AuthenticatorTransport {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}
