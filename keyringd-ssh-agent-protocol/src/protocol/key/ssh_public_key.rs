use super::SshPublicKeyBlob;
use super::{EcdsaP256PublicKey, SkEcdsaP256PublicKey, SshEllipticCurve, SshKeyAlgorithm};
use crate::protocol::codec::{SshAgentCodecError, SshReader, SshWriter};

/// A supported SSH public key, excluding certificate key formats.
/// Wire blobs are retained separately by `SshPublicKeyBlob`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SshPublicKey {
    EcdsaP256(EcdsaP256PublicKey),
    SkEcdsaP256(SkEcdsaP256PublicKey),
}

impl SshPublicKey {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SshAgentCodecError> {
        let mut reader = SshReader::new(bytes);
        let name = read_string(&mut reader, "key algorithm")?;
        let algorithm =
            SshKeyAlgorithm::decode(name).map_err(|_| SshAgentCodecError::InvalidUtf8 {
                field: "key algorithm",
            })?;
        match algorithm {
            SshKeyAlgorithm::EcdsaSha2NistP256 | SshKeyAlgorithm::SkEcdsaSha2NistP256 => {
                let key = Self::read_public_key_fields(&mut reader, &algorithm)?;
                let trailing_bytes = reader.remaining().len();
                reader
                    .finish()
                    .map_err(|_| SshAgentCodecError::TrailingData { trailing_bytes })?;
                Ok(key)
            }
            SshKeyAlgorithm::EcdsaSha2NistP256Certificate
            | SshKeyAlgorithm::SkEcdsaSha2NistP256Certificate => {
                Err(SshAgentCodecError::Unsupported {
                    field: "public key algorithm",
                })
            }
            SshKeyAlgorithm::Unsupported(_) => Err(SshAgentCodecError::Unsupported {
                field: "public key algorithm",
            }),
        }
    }

    pub fn from_ecdsa_pubkey(q: &[u8]) -> Result<Self, SshAgentCodecError> {
        EcdsaP256PublicKey::new(q).map(Self::EcdsaP256)
    }

    pub fn from_sk_ecdsa_pubkey(q: &[u8], application: &str) -> Result<Self, SshAgentCodecError> {
        SkEcdsaP256PublicKey::new(q, application.as_bytes()).map(Self::SkEcdsaP256)
    }

    pub fn algorithm(&self) -> SshKeyAlgorithm {
        match self {
            Self::EcdsaP256(_) => SshKeyAlgorithm::EcdsaSha2NistP256,
            Self::SkEcdsaP256(_) => SshKeyAlgorithm::SkEcdsaSha2NistP256,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = SshWriter::new();
        writer.write_string(self.algorithm().as_bytes());
        match self {
            Self::EcdsaP256(key) => {
                writer.write_string(key.curve().as_bytes());
                writer.write_string(key.q());
            }
            Self::SkEcdsaP256(key) => {
                writer.write_string(key.curve().as_bytes());
                writer.write_string(key.q());
                writer.write_string(key.application());
            }
        }
        writer.into_bytes()
    }

    pub fn into_key_blob(self) -> SshPublicKeyBlob {
        SshPublicKeyBlob::from_bytes(&self.to_bytes())
            .expect("encoded public keys have an algorithm")
    }

    /// Decode the algorithm-specific public key fields.
    fn read_public_key_fields(
        reader: &mut SshReader<'_>,
        algorithm: &SshKeyAlgorithm,
    ) -> Result<Self, SshAgentCodecError> {
        let curve = read_string(reader, "curve name")?;
        if SshEllipticCurve::from_bytes(curve) != Some(SshEllipticCurve::NistP256) {
            return Err(SshAgentCodecError::Unsupported {
                field: "curve name",
            });
        }
        let q = read_string(reader, "public key")?;
        match algorithm {
            SshKeyAlgorithm::EcdsaSha2NistP256 => Self::from_ecdsa_pubkey(q),
            SshKeyAlgorithm::SkEcdsaSha2NistP256 => {
                let application = read_string(reader, "application")?;
                SkEcdsaP256PublicKey::new(q, application).map(Self::SkEcdsaP256)
            }
            _ => Err(SshAgentCodecError::Unsupported {
                field: "public key algorithm",
            }),
        }
    }
}

fn read_string<'a>(
    reader: &mut SshReader<'a>,
    field: &'static str,
) -> Result<&'a [u8], SshAgentCodecError> {
    reader
        .read_string()
        .map_err(|_| SshAgentCodecError::Truncated { field })
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::elliptic_curve::sec1::ToSec1Point;

    #[test]
    fn rejects_unsupported_key_and_certificate_algorithms() {
        for algorithm in ["ssh-ed25519", "ssh-ed25519-cert-v01@openssh.com"] {
            let mut writer = SshWriter::new();
            writer.write_string(algorithm.as_bytes());
            writer.write_string(&[7; 32]);
            let bytes = writer.into_bytes();
            assert!(matches!(
                SshPublicKey::from_bytes(&bytes),
                Err(SshAgentCodecError::Unsupported { .. })
            ));
            let blob = SshPublicKeyBlob::from_bytes(&bytes).unwrap();
            assert_eq!(
                blob.algorithm(),
                SshKeyAlgorithm::Unsupported(algorithm.into())
            );
            assert_eq!(blob.to_bytes(), bytes);
            assert_eq!(
                SshPublicKeyBlob::from_bytes(&blob.to_bytes()).unwrap(),
                blob
            );
        }
    }

    keyringd_testkit::scenario! {
        group "SshPublicKey::from_ecdsa_pubkey" {
            case "Encode the ECDSA P-256 public key to SSH Wire format" {
                spec "https://www.rfc-editor.org/rfc/rfc5656.html#section-3.1"
                given "Prepare the ECDSA P-256 public key" {
                    let public_key = p256::SecretKey::from_slice(&[1; 32])
                        .unwrap()
                        .public_key()
                        .to_sec1_point(false);
                }
                when "Convert Key to SSH Public Key" {
                    let blob = SshPublicKey::from_ecdsa_pubkey(public_key.as_bytes()).unwrap().into_key_blob();
                    let encoded = blob.to_bytes();
                    let mut reader = SshReader::new(&encoded);
                    let algorithm = reader.read_string().unwrap();
                    let curve = reader.read_string().unwrap();
                    let point = reader.read_string().unwrap();
                }
                then "Encode Algorithm, Curve and Uncompressed Points" {
                    assert_eq!(algorithm, SshKeyAlgorithm::EcdsaSha2NistP256.as_bytes());
                    assert_eq!(curve, SshEllipticCurve::NistP256.as_bytes());
                    assert_eq!(point.len(), 65);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SshPublicKey::from_sk_ecdsa_pubkey" {
            case "Encode Security Key ECDSA P-256 public key to SSH Wire format" {
                spec "https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.u2f#L55-L64"
                given "Prepare ECDSA P-256 public key and SSH Application" {
                    let public_key = p256::SecretKey::from_slice(&[1; 32])
                        .unwrap()
                        .public_key()
                        .to_sec1_point(false);
                }
                when "Convert Key to Security Key SSH Public Key" {
                    let blob = SshPublicKey::from_sk_ecdsa_pubkey(public_key.as_bytes(), "ssh:").unwrap().into_key_blob();
                    let encoded = blob.to_bytes();
                    let mut reader = SshReader::new(&encoded);
                    let algorithm = reader.read_string().unwrap();
                    reader.read_string().unwrap();
                    reader.read_string().unwrap();
                    let application = reader.read_string().unwrap();
                }
                then "Encode Security Key Algorithm and Application" {
                    assert_eq!(algorithm, SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes());
                    assert_eq!(application, b"ssh:");
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "SshPublicKey::from_bytes" {
            case "Exposes key fields and preserves their original wire encoding" {
                why "Structured keys must not normalize Q or lose the SK application when re-encoded."
                verify "both SEC1 encodings and binary application bytes survive parsing" {
                    let public_key = p256::SecretKey::from_slice(&[1; 32]).unwrap().public_key();
                    for compressed in [false, true] {
                        let q = public_key.to_sec1_point(compressed);
                        for application in [None, Some(b"ssh:\0\xff".as_slice())] {
                            let mut writer = SshWriter::new();
                            writer.write_string(if application.is_some() {
                                SshKeyAlgorithm::SkEcdsaSha2NistP256.as_bytes()
                            } else {
                                SshKeyAlgorithm::EcdsaSha2NistP256.as_bytes()
                            });
                            writer.write_string(b"nistp256");
                            writer.write_string(q.as_bytes());
                            if let Some(application) = application {
                                writer.write_string(application);
                            }
                            let encoded = writer.into_bytes();
                            let decoded = SshPublicKey::from_bytes(&encoded).unwrap();
                            match &decoded {
                                SshPublicKey::EcdsaP256(key) => {
                                    assert_eq!(key.q(), q.as_bytes());
                                    assert_eq!(key.curve(), SshEllipticCurve::NistP256);
                                    assert!(application.is_none());
                                }
                                SshPublicKey::SkEcdsaP256(key) => {
                                    assert_eq!(key.q(), q.as_bytes());
                                    assert_eq!(key.curve(), SshEllipticCurve::NistP256);
                                    assert_eq!(Some(key.application()), application);
                                }
                            }
                            assert_eq!(decoded.to_bytes(), encoded);
                        }
                    }
                }
            }
            case "Reject a public key in an incorrect known format" {
                spec "https://www.rfc-editor.org/rfc/rfc5656.html#section-3.1"
                given "Prepare an ECDSA Key Blob that contains an unauthorized Curve Point" {
                    let mut writer = SshWriter::new();
                    writer.write_string(SshKeyAlgorithm::EcdsaSha2NistP256.as_bytes());
                    writer.write_string(SshEllipticCurve::NistP256.as_bytes());
                    writer.write_string(&[0x04; 65]);
                }
                when "Decode the public key" {
                    let decoded = SshPublicKey::from_bytes(&writer.into_bytes());
                }
                then "Returns InvalidField Error for Public_Key without accepting illegal Curve Points" {
                    assert!(matches!(
                        decoded,
                        Err(SshAgentCodecError::InvalidValue {
                            field: "public key",
                            ..
                        })
                    ));
                }
            }
            case "Reject extra data following a known public key" {
                why "Requires consumption of the public key blob and does not misidentify consolidated data as a valid single key"
                spec "https://www.rfc-editor.org/rfc/rfc5656.html#section-3.1"
                given "Add one unconsumed byte to the end of the complete ECDSA P-256 public key blob" {
                    let public_key = p256::SecretKey::from_slice(&[1; 32])
                        .unwrap()
                        .public_key()
                        .to_sec1_point(false);
                    let mut encoded_key = SshPublicKey::from_ecdsa_pubkey(public_key.as_bytes())
                        .unwrap().to_bytes();
                    encoded_key.push(0);
                }
                when "Decode the public key" {
                    let decoded = SshPublicKey::from_bytes(&encoded_key);
                }
                then "Returns a trailingdata error without accepting extra bytes" {
                    assert!(matches!(
                        decoded,
                        Err(SshAgentCodecError::TrailingData { trailing_bytes: 1 })
                    ));
                }
            }
        }
    }
}
