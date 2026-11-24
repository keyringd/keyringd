use keyringd_ssh_agent_protocol::SshSignatureAlgorithm;
use sha2::{Digest, Sha256};

use crate::model::passkey::CredentialId;
use keyringd_ssh_agent_protocol::SshPublicKey;

use crate::model::passkey::PasskeyError;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Passkey {
    signature_algorithm: SshSignatureAlgorithm,
    credential_id: CredentialId,
    attestation_response: Option<super::PasskeyAttestationResponse>,
    #[serde(serialize_with = "crate::util::base64url_serde::serialize")]
    user_handle: Vec<u8>,
    #[serde(serialize_with = "crate::util::base64url_serde::serialize")]
    public_key: Vec<u8>,
}

impl<'de> serde::Deserialize<'de> for Passkey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct PasskeyJson {
            signature_algorithm: SshSignatureAlgorithm,
            credential_id: CredentialId,
            attestation_response: Option<super::PasskeyAttestationResponse>,
            #[serde(deserialize_with = "crate::util::base64url_serde::deserialize")]
            user_handle: Vec<u8>,
            #[serde(deserialize_with = "crate::util::base64url_serde::deserialize")]
            public_key: Vec<u8>,
        }

        let data = PasskeyJson::deserialize(deserializer)?;
        let passkey = Self::new(
            data.signature_algorithm,
            data.credential_id,
            data.user_handle,
            data.public_key,
        )
        .map_err(serde::de::Error::custom)?;
        match data.attestation_response {
            Some(attestation_response) => passkey
                .with_attestation_response(attestation_response)
                .map_err(serde::de::Error::custom),
            None => Ok(passkey),
        }
    }
}

impl Passkey {
    /// RP ID used for Passkeys and the application of newly created Device Keys.
    pub const RP_ID: &str = "ssh:keyringd";
    /// Origin included in ClientDataJSON; independent of the RP ID.
    pub const WEB_AUTHN_ORIGIN: &str = "https://keyringd.com";

    pub const USER_HANDLE_LEN: usize = 32;

    pub(crate) fn new(
        signature_algorithm: SshSignatureAlgorithm,
        credential_id: CredentialId,
        user_handle: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Result<Self, PasskeyError> {
        Self::validate_signature_algorithm(&signature_algorithm)?;
        validate_user_handle(&user_handle)?;
        validate_public_key(&public_key)?;
        Ok(Self {
            signature_algorithm,
            credential_id,
            attestation_response: None,
            user_handle,
            public_key,
        })
    }

    pub fn attestation_response(&self) -> Option<&super::PasskeyAttestationResponse> {
        self.attestation_response.as_ref()
    }
    pub(crate) fn with_attestation_response(
        mut self,
        attestation_response: super::PasskeyAttestationResponse,
    ) -> Result<Self, PasskeyError> {
        if !attestation_response.matches(&self.credential_id, &self.public_key) {
            return Err(PasskeyError::AttestationMismatch);
        }
        self.attestation_response = Some(attestation_response);
        Ok(self)
    }

    pub fn signature_algorithm(&self) -> SshSignatureAlgorithm {
        self.signature_algorithm.clone()
    }

    pub(crate) fn validate_signature_algorithm(
        signature_algorithm: &SshSignatureAlgorithm,
    ) -> Result<(), PasskeyError> {
        if !matches!(
            signature_algorithm,
            SshSignatureAlgorithm::SkEcdsaSha2NistP256
                | SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256
        ) {
            Err(PasskeyError::UnsupportedSignatureAlgorithm)
        } else {
            Ok(())
        }
    }

    pub fn credential_id(&self) -> &CredentialId {
        &self.credential_id
    }

    pub fn user_handle(&self) -> &[u8] {
        &self.user_handle
    }

    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    /// Encode the public key and RP ID in SSH security-key public-key format.
    pub fn to_ssh_public_key(&self) -> SshPublicKey {
        SshPublicKey::from_sk_ecdsa_pubkey(&self.public_key, Self::RP_ID)
            .expect("Passkey guarantees a valid ECDSA P-256 public key")
    }

    /// Derive the certificate principal from the SSH public key, including its RP ID.
    pub(crate) fn ssh_certificate_principal(&self) -> String {
        const PREFIX: &str = "keyringd:";
        let blob = self.to_ssh_public_key();
        let digest = Sha256::digest(blob.to_bytes());
        let mut principal = String::with_capacity(PREFIX.len() + digest.len() * 2);
        principal.push_str(PREFIX);
        for byte in digest {
            use std::fmt::Write;
            write!(&mut principal, "{byte:02x}").expect("writing to a String cannot fail");
        }
        principal
    }
}

fn validate_user_handle(bytes: &[u8]) -> Result<(), PasskeyError> {
    if bytes.len() != Passkey::USER_HANDLE_LEN {
        Err(PasskeyError::InvalidUserHandleLength {
            expected: Passkey::USER_HANDLE_LEN,
            actual: bytes.len(),
        })
    } else {
        Ok(())
    }
}
fn validate_public_key(bytes: &[u8]) -> Result<(), PasskeyError> {
    p256::PublicKey::from_sec1_bytes(bytes).map_err(PasskeyError::InvalidPublicKey)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::elliptic_curve::sec1::ToSec1Point;

    keyringd_testkit::scenario! {
        group "Passkey::deserialize" {
            case "Rejects invalid public keys when restoring a Passkey" {
                verify "reject a serialized Passkey with an invalid P-256 point" {
                    use crate::test_support::ModelProfileFixture;
                    use base64::Engine;
                    let mut encoded = serde_json::to_value(ModelProfileFixture::passkey(
                        SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                    ))
                    .unwrap();
                    encoded["publicKey"] = base64::engine::general_purpose::URL_SAFE_NO_PAD
                        .encode([3; 65])
                        .into();
                    assert!(serde_json::from_value::<Passkey>(encoded).is_err());
                }
            }
        }
        group "Passkey::ssh_certificate_principal" {
            case "Derives a stable product-scoped principal from a Passkey" {
                given "a fixed Passkey and SSH relying party" {
                    let passkey = Passkey::new(
                        keyringd_ssh_agent_protocol::SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                        CredentialId::new(vec![1]),
                        vec![2; Passkey::USER_HANDLE_LEN],
                        p256::SecretKey::from_slice(&[3; 32])
                            .unwrap()
                            .public_key()
                            .to_sec1_point(false)
                            .as_bytes()
                            .to_vec(),
                    )
                    .unwrap();
                }
                when "the principal is derived twice from the same key and relying party" {
                    let first = passkey.ssh_certificate_principal();
                    let second = passkey.ssh_certificate_principal();
                }
                then "both derivations are identical and use the keyringd product prefix" {
                    assert_eq!(first, second);
                    assert_eq!(first, passkey.ssh_certificate_principal());
                    assert!(first.starts_with("keyringd:"));
                }
            }
        }
        group "Passkey::new" {
            case "Rejects an invalid Passkey public key" {

                given "a credential ID and a correctly sized user handle" {
                    let credential_id = CredentialId::new(vec![1]);
                    let user_handle = vec![2; Passkey::USER_HANDLE_LEN];
                }
                when "a Passkey is constructed with invalid SEC1 public-key bytes" {
                    let result = Passkey::new(
                        SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                        credential_id,
                        user_handle,
                        vec![3; 65],
                    );
                }
                then "construction fails with InvalidPublicKey" {
                    assert!(matches!(result, Err(PasskeyError::InvalidPublicKey(_))));
                }
            }

            case "Accepts only security-key and WebAuthn formats in construction and deserialization" {
                verify "reject plain ECDSA and unknown algorithms at both input boundaries" {
                    use crate::test_support::ModelProfileFixture;
                    for format in [
                        SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                        SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256,
                        SshSignatureAlgorithm::EcdsaSha2NistP256,
                        SshSignatureAlgorithm::Unsupported("unknown-signature@example.com".into()),
                    ] {
                        let result = Passkey::new(
                            format.clone(),
                            CredentialId::new(vec![1]),
                            vec![7; 32],
                            ModelProfileFixture::public_key(7),
                        );
                        let allowed = matches!(format, SshSignatureAlgorithm::SkEcdsaSha2NistP256 | SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256);
                        assert_eq!(result.is_ok(), allowed);
                        if let Err(error) = result {
                            assert!(matches!(error, PasskeyError::UnsupportedSignatureAlgorithm));
                        }
                        let original = ModelProfileFixture::passkey(SshSignatureAlgorithm::SkEcdsaSha2NistP256);
                        let mut encoded = serde_json::to_value(original).unwrap();
                        encoded["signatureAlgorithm"] = serde_json::to_value(format).unwrap();
                        assert_eq!(serde_json::from_value::<Passkey>(encoded).is_ok(), allowed);
                    }
                }
            }

            case "Enforces the user-handle length in construction and deserialization" {
                verify "accept exactly a 32-byte user handle at both input boundaries" {
                    use crate::test_support::ModelProfileFixture;
                    use base64::Engine;
                    for length in [0, 31, 32, 33] {
                        let result = Passkey::new(
                            SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                            CredentialId::new(vec![1]),
                            vec![7; length],
                            ModelProfileFixture::public_key(7),
                        );
                        assert_eq!(result.is_ok(), length == 32);
                        if let Err(error) = result {
                            assert!(
                                matches!(error, PasskeyError::InvalidUserHandleLength { expected: 32, actual } if actual == length)
                            );
                        }
                        let mut encoded = serde_json::to_value(ModelProfileFixture::passkey(
                            SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                        ))
                        .unwrap();
                        encoded["userHandle"] = base64::engine::general_purpose::URL_SAFE_NO_PAD
                            .encode(vec![7; length])
                            .into();
                        assert_eq!(
                            serde_json::from_value::<Passkey>(encoded).is_ok(),
                            length == 32
                        );
                    }
                }
            }
        }
        group "Passkey::to_ssh_public_key" {
            case "Uses the Passkey RP and public key for both supported signature formats" {
                verify "derive both SSH public keys with the same relying-party ID and curve point" {
                    use crate::test_support::ModelProfileFixture;
                    let bytes = ModelProfileFixture::public_key(7);
                    let expected = SshPublicKey::from_sk_ecdsa_pubkey(&bytes, "ssh:keyringd").unwrap();
                    for format in [
                        SshSignatureAlgorithm::SkEcdsaSha2NistP256,
                        SshSignatureAlgorithm::WebAuthnSkEcdsaSha2NistP256,
                    ] {
                        let passkey = ModelProfileFixture::passkey(format);
                        assert_eq!(passkey.to_ssh_public_key(), expected);
                    }
                }
            }
        }
    }
}
