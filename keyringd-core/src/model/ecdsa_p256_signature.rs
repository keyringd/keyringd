use crate::model::EcdsaP256SignatureError;
use keyringd_ssh_agent_protocol::EcdsaP256SshSignature;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EcdsaP256Signature {
    signature: p256::ecdsa::Signature,
}

const P256_RAW_SIG_LEN: usize = 64;

impl EcdsaP256Signature {
    pub fn from_raw(bytes: &[u8]) -> Result<Self, EcdsaP256SignatureError> {
        if bytes.len() != P256_RAW_SIG_LEN {
            return Err(EcdsaP256SignatureError::RawLength {
                actual: bytes.len(),
            });
        }
        let signature = p256::ecdsa::Signature::from_slice(bytes)
            .map_err(EcdsaP256SignatureError::RawScalars)?;
        Ok(Self { signature })
    }

    pub fn from_der(bytes: &[u8]) -> Result<Self, EcdsaP256SignatureError> {
        let signature =
            p256::ecdsa::Signature::from_der(bytes).map_err(EcdsaP256SignatureError::Der)?;
        Ok(Self { signature })
    }

    pub(crate) fn to_ssh_signature(&self) -> EcdsaP256SshSignature {
        let bytes = self.signature.to_bytes();
        EcdsaP256SshSignature::new(&bytes[..32], &bytes[32..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "EcdsaP256Signature::from_raw / from_der" {
            case "Preserves r and s when accepting raw or DER signatures" {
                verify "decode both representations to the same two P-256 scalars" {
                    let mut bytes = [1; 64];
                    bytes[32..].fill(2);
                    let expected = EcdsaP256Signature::from_raw(&bytes).unwrap();
                    let der = p256::ecdsa::Signature::from_slice(&bytes).unwrap().to_der();
                    assert_eq!(
                        EcdsaP256Signature::from_der(der.as_bytes()).unwrap(),
                        expected
                    );
                    let ssh = expected.to_ssh_signature();
                    assert_eq!(ssh.r(), &[1; 32]);
                    assert_eq!(ssh.s(), &[2; 32]);
                }
            }

            case "Rejects invalid raw lengths and malformed DER" {
                verify "reject short and long raw signatures and invalid DER sequences" {
                    for length in [0, 1, 63, 65] {
                        assert!(
                            matches!(EcdsaP256Signature::from_raw(&vec![1; length]), Err(EcdsaP256SignatureError::RawLength { actual }) if actual == length)
                        );
                    }
                    for bytes in [vec![], vec![0; 10], vec![0x30, 0x01, 0x00]] {
                        assert!(matches!(
                            EcdsaP256Signature::from_der(&bytes),
                            Err(EcdsaP256SignatureError::Der(_))
                        ));
                    }
                }
            }
            case "Rejects invalid ECDSA scalars in both raw and DER encodings" {
                why "A fixed raw length must not allow zero or out-of-range scalars that DER decoding rejects."
                verify "all constructors enforce P-256 signature scalar validity" {
                    let mut zero_r = [1; 64];
                    zero_r[..32].fill(0);
                    let mut zero_s = [1; 64];
                    zero_s[32..].fill(0);
                    for raw in [[0; 64], [0xff; 64], zero_r, zero_s] {
                        assert!(matches!(EcdsaP256Signature::from_raw(&raw),
                            Err(EcdsaP256SignatureError::RawScalars(_))));
                    }
                    for der in [
                        [0x30, 0x06, 0x02, 0x01, 0, 0x02, 0x01, 1],
                        [0x30, 0x06, 0x02, 0x01, 1, 0x02, 0x01, 0],
                    ] {
                        assert!(matches!(EcdsaP256Signature::from_der(&der),
                            Err(EcdsaP256SignatureError::Der(_))));
                    }
                }
            }
        }
    }
}
