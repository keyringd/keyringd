use crate::HybridDataChannel;
use crate::{HybridAdvertisement, HybridQrInitiatedTransaction};
use keyringd_fido::transport::hybrid::HybridPostHandshakeMessage;
use keyringd_fido::{CtapCommand, CtapStatusCode};
use snow::{HandshakeState, TransportState};
use std::collections::VecDeque;

/// A test data channel that simulates the Noise handshake and encrypted response on the smartphone side.
/// We will change the success or failure of the corresponding function and the termination notification, and verify the CTAP communication and the aftermath of the HYBRID.
pub(crate) struct ModelHybridChannel {
    handshake: Option<HandshakeState>,
    transport: Option<TransportState>,
    incoming: VecDeque<Vec<u8>>,
    get_info: Vec<u8>,
    supported_features: Vec<String>,
    pub(crate) closed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub(crate) shutdown_frames: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    pub(crate) fail_shutdown_send: bool,
}

impl ModelHybridChannel {
    pub(crate) fn new(
        transaction: &HybridQrInitiatedTransaction,
        advertisement: &HybridAdvertisement,
        get_info: Vec<u8>,
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
            get_info,
            supported_features: vec!["ctap".to_owned()],
            closed: Default::default(),
            shutdown_frames: Default::default(),
            fail_shutdown_send: false,
        }
    }

    pub(crate) fn with_supported_features(mut self, supported_features: Vec<String>) -> Self {
        self.supported_features = supported_features;
        self
    }

    fn enqueue_initial(&mut self) {
        let encoded = HybridPostHandshakeMessage::new(self.get_info.clone())
            .with_supported_features(self.supported_features.clone())
            .to_cbor_bytes()
            .unwrap();
        self.enqueue_plain(encoded);
    }

    fn enqueue_plain(&mut self, mut plaintext: Vec<u8>) {
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

impl HybridDataChannel for ModelHybridChannel {
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
            self.enqueue_initial();
        } else {
            let mut plaintext = vec![0; message.len()];
            let length = self
                .transport
                .as_mut()
                .unwrap()
                .read_message(&message, &mut plaintext)
                .unwrap();
            plaintext.truncate(length);
            crate::transport::hybrid::ctap::strip_padding(&mut plaintext).unwrap();
            if plaintext == [0] {
                self.shutdown_frames
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if self.fail_shutdown_send {
                    return Err(std::io::Error::other("injected shutdown failure"));
                }
            } else {
                assert_eq!(plaintext, [1, CtapCommand::GetAssertion.as_byte(), 0xa0]);
                self.enqueue_plain(vec![2, 0xa0]);
                self.enqueue_plain(vec![1, CtapStatusCode::Ok.as_byte(), 0xa0]);
            }
        }
        Ok(())
    }

    async fn receive(&mut self) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.incoming.pop_front())
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        self.closed.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}
