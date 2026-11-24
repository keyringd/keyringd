use super::DeviceKeyAlgorithm;
use keyringd_ssh_agent_protocol::{EcdsaP256PublicKey, SshAgentCodecError, SshPublicKey};

#[derive(Clone, Debug)]
pub enum DevicePublicKey {
    EcdsaP256(EcdsaP256PublicKey),
}

impl DevicePublicKey {
    pub fn from_ecdsa_p256(q: &[u8]) -> Result<Self, SshAgentCodecError> {
        EcdsaP256PublicKey::new(q).map(Self::EcdsaP256)
    }

    /// `Some(application)` produces an SK key; `None` produces an ordinary key.
    pub(crate) fn to_ssh_public_key(
        &self,
        application: Option<&str>,
    ) -> Result<SshPublicKey, SshAgentCodecError> {
        match (self, application) {
            (Self::EcdsaP256(key), None) => Ok(SshPublicKey::EcdsaP256(key.clone())),
            (Self::EcdsaP256(key), Some(application)) => {
                SshPublicKey::from_sk_ecdsa_pubkey(key.q(), application)
            }
        }
    }

    pub fn algorithm(&self) -> DeviceKeyAlgorithm {
        match self {
            Self::EcdsaP256(_) => DeviceKeyAlgorithm::EcdsaP256,
        }
    }

    pub fn public_key_bytes(&self) -> &[u8] {
        match self {
            Self::EcdsaP256(key) => key.q(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ModelProfileFixture;
    use keyringd_ssh_agent_protocol::SshKeyAlgorithm;

    keyringd_testkit::scenario! {
        group "DevicePublicKey::to_ssh_public_key" {
            case "Encodes ordinary keys or security keys with the supplied application" {
                verify "an application selects SK encoding and is preserved exactly" {
                    let bytes = ModelProfileFixture::public_key(11);
                    let key = DevicePublicKey::from_ecdsa_p256(&bytes).unwrap();
                    let plain = key.to_ssh_public_key(None).unwrap();
                    assert_eq!(
                        plain.algorithm(),
                        SshKeyAlgorithm::EcdsaSha2NistP256
                    );
                    assert_eq!(plain, SshPublicKey::from_ecdsa_pubkey(&bytes).unwrap());
                    for application in ["ssh:keyringd", "ssh:other"] {
                        let blob = key.to_ssh_public_key(Some(application)).unwrap();
                        assert_eq!(
                            blob.algorithm(),
                            SshKeyAlgorithm::SkEcdsaSha2NistP256
                        );
                        assert_eq!(
                            blob,
                            SshPublicKey::from_sk_ecdsa_pubkey(&bytes, application).unwrap()
                        );
                    }
                }
            }
            case "Rejects invalid P-256 points at construction" {
                verify "an invalid point cannot enter the typed key model" {
                    for bytes in [vec![], vec![0], vec![3; 65]] {
                        assert!(DevicePublicKey::from_ecdsa_p256(&bytes).is_err());
                    }
                }
            }
        }
        group "DevicePublicKey accessors" {
            case "Derives the algorithm from the variant and preserves the public key bytes" {
                verify "the variant determines the algorithm and the payload retains Q" {
                    let bytes = ModelProfileFixture::public_key(11);
                    let key = DevicePublicKey::from_ecdsa_p256(&bytes).unwrap();
                    assert!(matches!(key.algorithm(), DeviceKeyAlgorithm::EcdsaP256));
                    assert_eq!(key.public_key_bytes(), bytes);
                }
            }
        }
    }
}
