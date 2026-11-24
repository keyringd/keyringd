use crate::protocol::codec::{SshAgentCodecError, SshReader, SshWriter};
use crate::{
    EcdsaP256SshSignature, SkEcdsaP256SshSignature, SshSignatureAlgorithm,
    WebAuthnEcdsaP256SshSignature,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SshSignature {
    Ecdsa(EcdsaP256SshSignature),
    SkEcdsa(SkEcdsaP256SshSignature),
    WebAuthnSkEcdsa(WebAuthnEcdsaP256SshSignature),
    Unsupported { algorithm: String, data: Vec<u8> },
}

impl SshSignature {
    pub fn from_bytes(encoded: &[u8]) -> Result<Self, SshAgentCodecError> {
        let mut reader = SshReader::new(encoded);
        let algorithm_name = reader
            .read_string()
            .map_err(|_| SshAgentCodecError::Truncated {
                field: "signature algorithm",
            })?;
        let algorithm = SshSignatureAlgorithm::decode(algorithm_name).map_err(|_| {
            SshAgentCodecError::InvalidUtf8 {
                field: "signature algorithm",
            }
        })?;
        match algorithm {
            SshSignatureAlgorithm::SkEcdsaSha2NistP256 => Ok(Self::SkEcdsa(
                SkEcdsaP256SshSignature::from_bytes(reader.remaining())?,
            )),
            SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256 => Ok(Self::WebAuthnSkEcdsa(
                WebAuthnEcdsaP256SshSignature::from_bytes(reader.remaining())?,
            )),
            algorithm => {
                let data = reader
                    .read_string()
                    .map_err(|_| SshAgentCodecError::Truncated {
                        field: "signature data",
                    })?;
                if !reader.remaining().is_empty() {
                    return Err(SshAgentCodecError::TrailingData {
                        trailing_bytes: reader.remaining().len(),
                    });
                }
                match algorithm {
                    SshSignatureAlgorithm::EcdsaSha2NistP256 => {
                        Ok(Self::Ecdsa(EcdsaP256SshSignature::from_bytes(data)?))
                    }
                    SshSignatureAlgorithm::Unsupported(algorithm) => Ok(Self::Unsupported {
                        algorithm,
                        data: data.to_vec(),
                    }),
                    _ => unreachable!("SK algorithms handled above"),
                }
            }
        }
    }

    pub fn algorithm(&self) -> SshSignatureAlgorithm {
        match self {
            Self::Ecdsa(_) => SshSignatureAlgorithm::EcdsaSha2NistP256,
            Self::SkEcdsa(_) => SshSignatureAlgorithm::SkEcdsaSha2NistP256,
            Self::WebAuthnSkEcdsa(_) => SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256,
            Self::Unsupported { algorithm, .. } => {
                SshSignatureAlgorithm::Unsupported(algorithm.clone())
            }
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = SshWriter::new();
        writer.write_string(self.algorithm().as_bytes());
        match self {
            Self::Ecdsa(signature) => writer.write_string(&signature.to_bytes()),
            Self::SkEcdsa(signature) => writer.write_bytes(&signature.to_bytes()),
            Self::WebAuthnSkEcdsa(signature) => writer.write_bytes(&signature.to_bytes()),
            Self::Unsupported { data, .. } => writer.write_string(data),
        }
        writer.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signatures() -> Vec<SshSignature> {
        let ecdsa = EcdsaP256SshSignature::new([0, 0x80], [2]);
        vec![
            SshSignature::Ecdsa(ecdsa.clone()),
            SshSignature::SkEcdsa(SkEcdsaP256SshSignature::new(ecdsa.clone(), 5, 42)),
            SshSignature::WebAuthnSkEcdsa(WebAuthnEcdsaP256SshSignature::new(
                ecdsa,
                0x85,
                43,
                "https://example.com".into(),
                b"client data".to_vec(),
                vec![0xa1, 0x61, b'x', 0xf5],
            )),
            SshSignature::Unsupported {
                algorithm: "future@example.com".into(),
                data: vec![1, 2, 3],
            },
        ]
    }

    keyringd_testkit::scenario! {
        group "SshSignature::from_bytes" {
            case "Round trips typed signatures and preserves unknown algorithms" {
                verify "round-trip each typed signature and retain the opaque unknown algorithm" {
                    for signature in signatures() {
                        let encoded = signature.to_bytes();
                        let decoded = SshSignature::from_bytes(&encoded).unwrap();
                        assert_eq!(decoded, signature);
                        assert_eq!(decoded.to_bytes(), encoded);
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshSignature::from_bytes" {
            case "Rejects every truncation of each signature format" {
                verify "return a parse error at every incomplete signature length" {
                    for signature in signatures() {
                        let encoded = signature.to_bytes();
                        for length in 0..encoded.len() {
                            assert!(
                                SshSignature::from_bytes(&encoded[..length]).is_err(),
                                "accepted truncated {:?} at {}",
                                signature.algorithm(),
                                length
                            );
                        }
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshSignature::from_bytes" {
            case "Rejects trailing data for each signature format" {
                verify "return a parse error when any signature payload has trailing bytes" {
                    for signature in signatures() {
                        let mut encoded = signature.to_bytes();
                        encoded.push(0);
                        assert!(matches!(
                            SshSignature::from_bytes(&encoded),
                            Err(SshAgentCodecError::TrailingData { .. })
                        ));
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "EcdsaP256SshSignature::from_bytes" {
            case "Rejects negative noncanonical or oversized ecdsa components" {
                verify "reject invalid sign, padding, and size in ECDSA scalars" {
                    for invalid_r in [vec![0x80], vec![0], vec![0, 1], vec![1; 33]] {
                        let mut data = SshWriter::new();
                        data.write_string(&invalid_r);
                        data.write_string(&[2]);
                        assert!(matches!(
                            EcdsaP256SshSignature::from_bytes(&data.into_bytes()),
                            Err(SshAgentCodecError::InvalidValue { .. })
                        ));
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshSignature::to_bytes" {
            case "Rejects trailing data inside the nested ecdsa signature" {
                verify "reject unread bytes within the inner ECDSA signature payload" {
                    let mut ecdsa = EcdsaP256SshSignature::new([1], [2]).to_bytes();
                    ecdsa.push(0);
                    let mut sk = SshWriter::new();
                    sk.write_string(&ecdsa);
                    sk.write_u8(5);
                    sk.write_u32(42);
                    assert!(matches!(
                        SkEcdsaP256SshSignature::from_bytes(&sk.into_bytes()),
                        Err(SshAgentCodecError::TrailingData { .. })
                    ));
                }
            }
        }
    }
}
