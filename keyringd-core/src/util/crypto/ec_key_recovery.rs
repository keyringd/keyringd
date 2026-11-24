use ecdsa::RecoveryId;
use p256::ecdsa::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

use super::RecoveryError;

pub(crate) fn candidates(
    signed_data: &[u8],
    signature_der: &[u8],
) -> Result<Vec<Vec<u8>>, RecoveryError> {
    let prehash = Sha256::digest(signed_data);
    let signature = Signature::from_der(signature_der).map_err(RecoveryError::InvalidSignature)?;

    const P256_RECOVERY_ID_COUNT: u8 = 2;
    let mut result = Vec::new();
    for id in 0..P256_RECOVERY_ID_COUNT {
        let recovery_id = RecoveryId::try_from(id)
            .map_err(|error| RecoveryError::RecoveryFailed(Some(error.into())))?;
        if let Ok(vk) = VerifyingKey::recover_from_prehash(&prehash, &signature, recovery_id) {
            result.push(vk.to_sec1_point(false).as_bytes().to_vec());
        }
    }
    if result.is_empty() {
        return Err(RecoveryError::RecoveryFailed(None));
    }
    Ok(result)
}

pub(crate) fn identify(
    candidates: &[Vec<u8>],
    signed_data: &[u8],
    signature_der: &[u8],
) -> Result<Vec<u8>, RecoveryError> {
    let prehash = Sha256::digest(signed_data);
    let signature = Signature::from_der(signature_der).map_err(RecoveryError::InvalidSignature)?;

    for candidate in candidates {
        let vk = VerifyingKey::from_sec1_bytes(candidate)
            .map_err(|error| RecoveryError::RecoveryFailed(Some(error.into())))?;
        if ecdsa::signature::hazmat::PrehashVerifier::verify_prehash(&vk, &prehash, &signature)
            .is_ok()
        {
            return Ok(candidate.clone());
        }
    }
    Err(RecoveryError::AmbiguousRecovery)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecdsa::signature::hazmat::PrehashSigner;
    use p256::ecdsa::SigningKey;
    use p256::elliptic_curve::Generate;

    fn sign(signing_key: &SigningKey, signed_data: &[u8]) -> Vec<u8> {
        let digest = Sha256::digest(signed_data);
        let (signature, _) = signing_key.sign_prehash(&digest).unwrap();
        signature.to_der().to_bytes().to_vec()
    }

    keyringd_testkit::scenario! {
        group "identify" {
            case "Identifies the original public key using a second signature" {
                given "a P-256 key pair and the expected uncompressed SEC1 public key" {
                    let signing_key = SigningKey::generate();
                    let verifying_key = VerifyingKey::from(&signing_key);
                    let expected = verifying_key.to_sec1_point(false).as_bytes().to_vec();
                }
                when "public-key candidates are recovered from a signature over the first message" {
                    let sig1 = sign(&signing_key, b"message1");
                    let cands = candidates(b"message1", &sig1).unwrap();
                }
                then "Two public key candidates are restored" {
                    assert!(cands.len() == 2);
                }
                when "the candidates are checked against a signature over a different message" {
                    let sig2 = sign(&signing_key, b"message2");
                    let identified = identify(&cands, b"message2", &sig2).unwrap();
                }
                then "the candidate that produced both signatures is returned as the original public key" {
                    assert_eq!(identified, expected);
                }
            }
        }
        group "candidates" {
            case "Recovers both possible P-256 public keys from one signature" {
                given "a P-256 signature over a known message" {
                    let signing_key = SigningKey::generate();
                    let sig = sign(&signing_key, b"test");
                }
                when "public-key recovery is attempted without a second signature" {
                    let cands = candidates(b"test", &sig).unwrap();
                }
                then "both mathematically valid public-key candidates are returned" {
                    assert_eq!(cands.len(), 2);
                }
            }
        }
    }
}
