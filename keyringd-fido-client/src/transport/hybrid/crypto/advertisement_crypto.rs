use super::{HybridCryptoError, HybridKeyPurpose, HybridSecret64};

use aes::cipher::{BlockCipherDecrypt, KeyInit};
use aes::{Aes256, Block};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

pub(crate) fn derive(
    secret: &[u8],
    salt: Option<&[u8]>,
    purpose: HybridKeyPurpose,
) -> Result<HybridSecret64, HybridCryptoError> {
    let mut output = [0u8; 64];
    Hkdf::<Sha256>::new(salt, secret)
        .expand(&[purpose as u8, 0, 0, 0], &mut output)
        .map_err(|_| HybridCryptoError::KeyDerivation)?;
    Ok(HybridSecret64(output))
}

pub(crate) fn decrypt_advertisement(
    eid_key: &HybridSecret64,
    advertisement: &[u8],
) -> Result<[u8; 16], HybridCryptoError> {
    if advertisement.len() != 20 {
        return Err(HybridCryptoError::InvalidAdvertisementLength(
            advertisement.len(),
        ));
    }
    let encrypted = &advertisement[..16];
    let tag = &advertisement[16..20];
    let mut mac = Hmac::<Sha256>::new_from_slice(&eid_key.as_bytes()[32..])
        .map_err(|_| HybridCryptoError::KeyDerivation)?;
    mac.update(encrypted);
    let expected = mac.finalize().into_bytes();
    if !bool::from(expected[..4].ct_eq(tag)) {
        return Err(HybridCryptoError::AdvertisementAuthentication);
    }
    let cipher = Aes256::new_from_slice(&eid_key.as_bytes()[..32])
        .map_err(|_| HybridCryptoError::KeyDerivation)?;
    let mut block = Block::try_from(encrypted)
        .expect("encrypted advertisement prefix is exactly one AES block");
    cipher.decrypt_block(&mut block);
    if block[0] != 0 {
        return Err(HybridCryptoError::AdvertisementReservedBits);
    }
    Ok(block.into())
}

#[cfg(test)]
mod tests {

    use super::*;

    keyringd_testkit::scenario! {
        group "decrypt_advertisement" {
            case "Decrypts java authenticator advertisement vector" {
                verify "decode the Java fixture vector into its expected advertisement fields" {
                    // Shared with HybridQrPayloadTest in webauthn4j-ctap-authenticator.
                    let qr_secret = hex_literal::hex!("000102030405060708090a0b0c0d0e0f");
                    let advertisement = hex_literal::hex!("35d007a5bae78986c164f4666e8421909bd69006");
                    let expected = hex_literal::hex!("000102030405060708090aaabbcc0000");
                    let eid_key = derive(&qr_secret, None, HybridKeyPurpose::Eid).unwrap();

                    assert_eq!(
                        decrypt_advertisement(&eid_key, &advertisement).unwrap(),
                        expected
                    );
                }
            }
        }
    }
}
