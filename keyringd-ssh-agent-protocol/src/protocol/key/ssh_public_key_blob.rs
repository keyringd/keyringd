use super::SshKeyAlgorithm;
use crate::SshAgentCodecError;
use crate::protocol::codec::SshReader;

/// Original wire bytes of an SSH public key or certificate.
/// Only the leading algorithm string is checked; key-specific fields are opaque.
/// Use `SshPublicKey::from_bytes` or `SshCertificate::from_bytes` to parse them.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SshPublicKeyBlob(Vec<u8>);

impl SshPublicKeyBlob {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SshAgentCodecError> {
        read_algorithm(bytes)?;
        Ok(Self(bytes.to_vec()))
    }

    pub fn algorithm(&self) -> SshKeyAlgorithm {
        read_algorithm(&self.0).expect("SshPublicKeyBlob checks its algorithm string")
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.clone()
    }
}

fn read_algorithm(bytes: &[u8]) -> Result<SshKeyAlgorithm, SshAgentCodecError> {
    let mut reader = SshReader::new(bytes);
    let name = reader
        .read_string()
        .map_err(|_| SshAgentCodecError::Truncated {
            field: "key algorithm",
        })?;
    SshKeyAlgorithm::decode(name).map_err(|_| SshAgentCodecError::InvalidUtf8 {
        field: "key algorithm",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::codec::SshWriter;

    #[test]
    fn preserves_wire_bytes_without_parsing_algorithm_specific_fields() {
        for algorithm in [
            "ecdsa-sha2-nistp256",
            "sk-ecdsa-sha2-nistp256@openssh.com",
            "ecdsa-sha2-nistp256-cert-v01@openssh.com",
            "ssh-ed25519",
            "future-key@example.com",
        ] {
            for data in [b"".as_slice(), b"\x00\xffopaque data"] {
                let mut writer = SshWriter::new();
                writer.write_string(algorithm.as_bytes());
                writer.write_bytes(data);
                let bytes = writer.into_bytes();
                let blob = SshPublicKeyBlob::from_bytes(&bytes).unwrap();
                assert_eq!(blob.algorithm().as_str(), algorithm);
                assert_eq!(blob.as_bytes(), bytes);
                assert_eq!(blob.to_bytes(), bytes);
                assert_eq!(SshPublicKeyBlob::from_bytes(blob.as_bytes()).unwrap(), blob);
            }
        }
    }

    #[test]
    fn rejects_a_missing_truncated_or_non_utf8_algorithm() {
        for bytes in [
            b"".as_slice(),
            b"\x00\x00\x00",
            b"\x00\x00\x00\x02x",
            b"\x00\x00\x00\x01\xff",
        ] {
            assert!(SshPublicKeyBlob::from_bytes(bytes).is_err());
        }
    }
}
