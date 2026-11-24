use crate::HybridNoiseError;

const MAX_MESSAGE_SIZE: usize = 1024 * 1024;
pub(crate) const PADDING_GRANULARITY: usize = 32;
pub(crate) const CTAP_MESSAGE_TYPE: u8 = 1;
const SHUTDOWN_MESSAGE_TYPE: u8 = 0;

pub(crate) fn pad_plaintext(plaintext: &mut Vec<u8>) -> Result<(), HybridNoiseError> {
    if plaintext.is_empty() || plaintext.len() > MAX_MESSAGE_SIZE {
        return Err(HybridNoiseError::FrameTooLarge);
    }
    let padding = PADDING_GRANULARITY - (plaintext.len() % PADDING_GRANULARITY);
    let unpadded_length = plaintext.len();
    plaintext.resize(unpadded_length + padding, 0);
    *plaintext.last_mut().expect("padding always present") = (padding - 1) as u8;
    Ok(())
}

pub(crate) fn strip_padding(message: &mut Vec<u8>) -> Result<(), HybridNoiseError> {
    let padding = usize::from(*message.last().ok_or(HybridNoiseError::InvalidFrame)?);
    let unpadded = message
        .len()
        .checked_sub(padding + 1)
        .ok_or(HybridNoiseError::InvalidFrame)?;
    message.truncate(unpadded);
    Ok(())
}

/// Holds the cryptographic state established by the Noise handshake and encrypts and decrypts Hybrid messages.
/// No connection search or WebSocket I/O.
pub struct HybridNoiseSession {
    pub(super) transport: snow::TransportState,
}

impl HybridNoiseSession {
    pub fn handshake(
        private_key: &[u8; 32],
        psk: &[u8; 32],
    ) -> Result<snow::HandshakeState, HybridNoiseError> {
        Ok(
            snow::Builder::new("Noise_KNpsk0_P256_AESGCM_SHA256".parse()?)
                .prologue(&[1])?
                .local_private_key(private_key)?
                .psk(0, psk)?
                .build_initiator()?,
        )
    }

    pub fn from_handshake(
        mut handshake: snow::HandshakeState,
        response: &[u8],
    ) -> Result<Self, HybridNoiseError> {
        let mut payload = vec![0; 1024];
        handshake.read_message(response, &mut payload)?;
        if !handshake.is_handshake_finished() {
            return Err(HybridNoiseError::HandshakeIncomplete);
        }
        Ok(Self {
            transport: handshake.into_transport_mode()?,
        })
    }

    pub fn encrypt_ctap(&mut self, message: &[u8]) -> Result<Vec<u8>, HybridNoiseError> {
        if message.len() >= MAX_MESSAGE_SIZE {
            return Err(HybridNoiseError::FrameTooLarge);
        }
        let mut plaintext = Vec::with_capacity(1 + message.len() + PADDING_GRANULARITY);
        plaintext.push(CTAP_MESSAGE_TYPE);
        plaintext.extend_from_slice(message);
        self.encrypt_frame(plaintext)
    }

    pub fn encrypt_shutdown(&mut self) -> Result<Vec<u8>, HybridNoiseError> {
        self.encrypt_frame(vec![SHUTDOWN_MESSAGE_TYPE])
    }

    pub(super) fn encrypt_frame(
        &mut self,
        mut plaintext: Vec<u8>,
    ) -> Result<Vec<u8>, HybridNoiseError> {
        pad_plaintext(&mut plaintext)?;
        let mut encrypted = vec![0u8; plaintext.len() + 16];
        let length = self.transport.write_message(&plaintext, &mut encrypted)?;
        encrypted.truncate(length);
        Ok(encrypted)
    }

    pub fn decrypt(&mut self, encrypted: &[u8]) -> Result<Vec<u8>, HybridNoiseError> {
        if encrypted.len() > MAX_MESSAGE_SIZE + 64 {
            return Err(HybridNoiseError::FrameTooLarge);
        }
        let mut plaintext = vec![0u8; encrypted.len()];
        let length = self.transport.read_message(encrypted, &mut plaintext)?;
        plaintext.truncate(length);
        strip_padding(&mut plaintext)?;
        Ok(plaintext)
    }

    #[cfg(test)]
    pub fn decrypt_ctap(&mut self, encrypted: &[u8]) -> Result<Vec<u8>, HybridNoiseError> {
        let plaintext = self.decrypt(encrypted)?;
        let (&message_type, payload) = plaintext
            .split_first()
            .ok_or(HybridNoiseError::InvalidFrame)?;
        if message_type != CTAP_MESSAGE_TYPE || payload.is_empty() {
            return Err(HybridNoiseError::InvalidFrame);
        }
        Ok(payload.to_vec())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    keyringd_testkit::scenario! {
        group "pad_plaintext" {
            case "Accepts a one-mebibyte hybrid plaintext and rejects the next byte" {
                why "The one-mebibyte cap is a local allocation and overflow guard taken from the reference implementation, not a normative Hybrid message-size requirement."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"
                given "plaintext values at and one byte beyond the hybrid transport limit" {
                    let mut maximum = vec![0; MAX_MESSAGE_SIZE];
                    let mut oversized = vec![0; MAX_MESSAGE_SIZE + 1];
                }
                when "the transport applies its authenticated-message padding" {
                    let maximum_result = pad_plaintext(&mut maximum);
                    let oversized_result = pad_plaintext(&mut oversized);
                }
                then "exactly one MiB is accepted while a larger plaintext is rejected" {
                    maximum_result.unwrap();
                    assert_eq!(maximum.len(), MAX_MESSAGE_SIZE + PADDING_GRANULARITY);
                    assert!(matches!(
                        oversized_result,
                        Err(HybridNoiseError::FrameTooLarge)
                    ));
                }
            }
        }
    }
}
