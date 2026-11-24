#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SshSignatureAlgorithm {
    EcdsaSha2NistP256,
    SkEcdsaSha2NistP256,
    WebAuthnSkEcdsaSha2NistP256,
    Unsupported(String),
}

impl SshSignatureAlgorithm {
    pub fn decode(name: &[u8]) -> Result<Self, std::str::Utf8Error> {
        let name = std::str::from_utf8(name)?;
        Ok(match name {
            "ecdsa-sha2-nistp256" => Self::EcdsaSha2NistP256,
            "sk-ecdsa-sha2-nistp256@openssh.com" => Self::SkEcdsaSha2NistP256,
            "webauthn-sk-ecdsa-sha2-nistp256@openssh.com" => Self::WebAuthnSkEcdsaSha2NistP256,
            name => Self::Unsupported(name.to_owned()),
        })
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::EcdsaSha2NistP256 => "ecdsa-sha2-nistp256",
            Self::SkEcdsaSha2NistP256 => "sk-ecdsa-sha2-nistp256@openssh.com",
            Self::WebAuthnSkEcdsaSha2NistP256 => "webauthn-sk-ecdsa-sha2-nistp256@openssh.com",
            Self::Unsupported(name) => name,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl std::fmt::Display for SshSignatureAlgorithm {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl serde::Serialize for SshSignatureAlgorithm {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for SshSignatureAlgorithm {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::decode(name.as_bytes()).map_err(serde::de::Error::custom)
    }
}
