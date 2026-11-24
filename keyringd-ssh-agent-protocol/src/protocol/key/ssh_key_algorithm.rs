/// Algorithm identifier carried by a public-key or certificate blob.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SshKeyAlgorithm {
    EcdsaSha2NistP256,
    EcdsaSha2NistP256Certificate,
    SkEcdsaSha2NistP256,
    SkEcdsaSha2NistP256Certificate,
    Unsupported(String),
}

impl SshKeyAlgorithm {
    pub fn decode(name: &[u8]) -> Result<Self, std::str::Utf8Error> {
        let name = std::str::from_utf8(name)?;
        Ok(match name {
            "ecdsa-sha2-nistp256" => Self::EcdsaSha2NistP256,
            "ecdsa-sha2-nistp256-cert-v01@openssh.com" => Self::EcdsaSha2NistP256Certificate,
            "sk-ecdsa-sha2-nistp256-cert-v01@openssh.com" => Self::SkEcdsaSha2NistP256Certificate,
            "sk-ecdsa-sha2-nistp256@openssh.com" => Self::SkEcdsaSha2NistP256,
            name => Self::Unsupported(name.to_owned()),
        })
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::EcdsaSha2NistP256 => "ecdsa-sha2-nistp256",
            Self::EcdsaSha2NistP256Certificate => "ecdsa-sha2-nistp256-cert-v01@openssh.com",
            Self::SkEcdsaSha2NistP256Certificate => "sk-ecdsa-sha2-nistp256-cert-v01@openssh.com",
            Self::SkEcdsaSha2NistP256 => "sk-ecdsa-sha2-nistp256@openssh.com",
            Self::Unsupported(name) => name,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl std::fmt::Display for SshKeyAlgorithm {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}
