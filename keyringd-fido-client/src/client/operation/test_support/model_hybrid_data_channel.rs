use crate::*;
use crate::{HybridAdvertisement, HybridDataChannel};

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// A test data channel that bridges the CTAP request/response of the HYBRID frame and authenticator model.
pub(crate) struct ModelHybridDataChannel {
    pub(crate) handshake: Option<snow::HandshakeState>,
    pub(crate) transport: Option<snow::TransportState>,
    pub(crate) incoming: VecDeque<Vec<u8>>,
    pub(crate) authenticator: Arc<keyringd_test_authenticator::ModelCtapAuthenticator>,
    pub(crate) closed_channels: Arc<AtomicUsize>,
    pub(crate) shutdown_frames: Arc<AtomicUsize>,
    pub(crate) ctap_entered: Option<Arc<AtomicBool>>,
    pub(crate) resume_ctap: Option<Arc<tokio::sync::Notify>>,
    pub(crate) closed: bool,
}

impl ModelHybridDataChannel {
    pub(crate) fn new(
        transaction: &HybridQrInitiatedTransaction,
        advertisement: &HybridAdvertisement,
        authenticator: Arc<keyringd_test_authenticator::ModelCtapAuthenticator>,
        closed_channels: Arc<AtomicUsize>,
        shutdown_frames: Arc<AtomicUsize>,
        ctap_entered: Option<Arc<AtomicBool>>,
        resume_ctap: Option<Arc<tokio::sync::Notify>>,
    ) -> Self {
        let psk = transaction.psk(advertisement.plaintext()).unwrap();
        let handshake = snow::Builder::new("Noise_KNpsk0_P256_AESGCM_SHA256".parse().unwrap())
            .prologue(&[1])
            .unwrap()
            .remote_public_key(&transaction.noise_public_key_sec1())
            .unwrap()
            .psk(0, &psk)
            .unwrap()
            .build_responder()
            .unwrap();
        Self {
            handshake: Some(handshake),
            transport: None,
            incoming: VecDeque::new(),
            authenticator,
            closed_channels,
            shutdown_frames,
            ctap_entered,
            resume_ctap,
            closed: false,
        }
    }

    fn enqueue_encrypted(&mut self, mut plaintext: Vec<u8>) {
        let padding = 32 - (plaintext.len() % 32);
        let unpadded = plaintext.len();
        plaintext.resize(unpadded + padding, 0);
        *plaintext.last_mut().unwrap() = (padding - 1) as u8;
        let mut encrypted = vec![0; plaintext.len() + 16];
        let length = self
            .transport
            .as_mut()
            .unwrap()
            .write_message(&plaintext, &mut encrypted)
            .unwrap();
        encrypted.truncate(length);
        self.incoming.push_back(encrypted);
    }
}

impl HybridDataChannel for ModelHybridDataChannel {
    type Error = std::io::Error;

    async fn send(&mut self, message: Vec<u8>) -> Result<(), Self::Error> {
        if let Some(mut handshake) = self.handshake.take() {
            let mut ignored = [0; 1024];
            handshake.read_message(&message, &mut ignored).unwrap();
            let mut response = vec![0; 1024];
            let length = handshake.write_message(&[], &mut response).unwrap();
            response.truncate(length);
            self.incoming.push_back(response);
            self.transport = Some(handshake.into_transport_mode().unwrap());
            let get_info = self
                .authenticator
                .transmit(keyringd_fido::CtapCommand::GetInfo, &[])
                .unwrap();
            let initial = keyringd_fido::transport::hybrid::HybridPostHandshakeMessage::new(
                get_info[1..].to_vec(),
            )
            .to_cbor_bytes()
            .unwrap();
            self.enqueue_encrypted(initial);
            return Ok(());
        }

        let mut plaintext = vec![0; message.len()];
        let length = self
            .transport
            .as_mut()
            .unwrap()
            .read_message(&message, &mut plaintext)
            .unwrap();
        plaintext.truncate(length);
        crate::transport::hybrid::ctap::strip_padding(&mut plaintext).unwrap();
        let (&message_type, payload) = plaintext.split_first().unwrap();
        match message_type {
            0 if payload.is_empty() => {
                self.closed = true;
                self.shutdown_frames.fetch_add(1, Ordering::SeqCst);
            }
            1 if !payload.is_empty() => {
                if let Some(entered) = &self.ctap_entered {
                    entered.store(true, Ordering::SeqCst);
                }
                if let Some(resume) = &self.resume_ctap {
                    resume.notified().await;
                }
                let response = self
                    .authenticator
                    .transmit(
                        keyringd_fido::CtapCommand::from_byte(payload[0]),
                        &payload[1..],
                    )
                    .unwrap();
                let mut framed = vec![1];
                framed.extend_from_slice(&response);
                self.enqueue_encrypted(framed);
            }
            _ => return Err(std::io::Error::other("invalid model Hybrid frame")),
        }
        Ok(())
    }

    async fn receive(&mut self) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.incoming.pop_front())
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        if !self.closed {
            self.closed = true;
        }
        self.closed_channels.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
