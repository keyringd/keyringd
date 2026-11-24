use aes::cipher::{BlockModeDecrypt, BlockModeEncrypt, KeyIvInit, block_padding::NoPadding};
use hkdf::Hkdf;
use hmac::{Hmac, KeyInit as HmacKeyInit, Mac};
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

use crate::{PinUvAuthCryptoError, PinUvAuthProtocol};

pub(crate) fn derive_secret(
    protocol: PinUvAuthProtocol,
    ecdh_secret: &[u8],
) -> Result<Vec<u8>, PinUvAuthCryptoError> {
    match protocol {
        PinUvAuthProtocol::One => Ok(Sha256::digest(ecdh_secret).to_vec()),
        PinUvAuthProtocol::Two => {
            let hkdf = Hkdf::<Sha256>::new(Some(&[0; 32]), ecdh_secret);
            let mut secret = vec![0; 64];
            hkdf.expand(b"CTAP2 HMAC key", &mut secret[..32])
                .map_err(|_| PinUvAuthCryptoError::KeyDerivation)?;
            hkdf.expand(b"CTAP2 AES key", &mut secret[32..])
                .map_err(|_| PinUvAuthCryptoError::KeyDerivation)?;
            Ok(secret)
        }
    }
}

pub(crate) fn encrypt(
    protocol: PinUvAuthProtocol,
    secret: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, PinUvAuthCryptoError> {
    if !plaintext.len().is_multiple_of(16) {
        return Err(PinUvAuthCryptoError::InvalidLength {
            protocol: protocol.version(),
        });
    }
    let (key, iv, include_iv) = match protocol {
        PinUvAuthProtocol::One if secret.len() == 32 => (secret, [0u8; 16], false),
        PinUvAuthProtocol::Two if secret.len() == 64 => {
            let mut iv = [0u8; 16];
            OsRng.fill_bytes(&mut iv);
            (&secret[32..], iv, true)
        }
        _ => {
            return Err(PinUvAuthCryptoError::InvalidLength {
                protocol: protocol.version(),
            });
        }
    };
    let encrypted = cbc::Encryptor::<aes::Aes256>::new_from_slices(key, &iv)
        .map_err(|_| PinUvAuthCryptoError::Encryption)?
        .encrypt_padded_vec::<NoPadding>(plaintext);
    if include_iv {
        let mut output = Vec::with_capacity(16 + encrypted.len());
        output.extend_from_slice(&iv);
        output.extend_from_slice(&encrypted);
        Ok(output)
    } else {
        Ok(encrypted)
    }
}

pub(crate) fn decrypt(
    protocol: PinUvAuthProtocol,
    secret: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, PinUvAuthCryptoError> {
    let (key, iv, ciphertext) = match protocol {
        PinUvAuthProtocol::One if secret.len() == 32 && ciphertext.len().is_multiple_of(16) => {
            (secret, [0u8; 16], ciphertext)
        }
        PinUvAuthProtocol::Two
            if secret.len() == 64
                && ciphertext.len() >= 16
                && ciphertext.len().is_multiple_of(16) =>
        {
            let mut iv = [0u8; 16];
            iv.copy_from_slice(&ciphertext[..16]);
            (&secret[32..], iv, &ciphertext[16..])
        }
        _ => {
            return Err(PinUvAuthCryptoError::InvalidLength {
                protocol: protocol.version(),
            });
        }
    };
    cbc::Decryptor::<aes::Aes256>::new_from_slices(key, &iv)
        .map_err(|_| PinUvAuthCryptoError::Decryption)?
        .decrypt_padded_vec::<NoPadding>(ciphertext)
        .map_err(|_| PinUvAuthCryptoError::Decryption)
}

pub(crate) fn authenticate(
    protocol: PinUvAuthProtocol,
    secret: &[u8],
    message: &[u8],
) -> Result<Vec<u8>, PinUvAuthCryptoError> {
    let key = match protocol {
        PinUvAuthProtocol::One if matches!(secret.len(), 16 | 32) => secret,
        PinUvAuthProtocol::Two if secret.len() == 64 => &secret[..32],
        PinUvAuthProtocol::Two if secret.len() == 32 => secret,
        _ => {
            return Err(PinUvAuthCryptoError::InvalidLength {
                protocol: protocol.version(),
            });
        }
    };
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key).map_err(|_| PinUvAuthCryptoError::KeyDerivation)?;
    mac.update(message);
    let output = mac.finalize().into_bytes();
    Ok(match protocol {
        PinUvAuthProtocol::One => output[..16].to_vec(),
        PinUvAuthProtocol::Two => output.to_vec(),
    })
}
