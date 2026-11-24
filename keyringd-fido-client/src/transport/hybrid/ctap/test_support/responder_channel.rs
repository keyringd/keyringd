use crate::{HybridDataChannel, HybridQrInitiatedTransaction};
use std::collections::VecDeque;
use std::convert::Infallible;

/// A session test channel that simulates handshake and cipher communications as the response of Noise.
pub(crate) struct ResponderChannel {
    handshake: Option<snow::HandshakeState>,
    pub(crate) transport: Option<snow::TransportState>,
    incoming: VecDeque<Vec<u8>>,
}

impl ResponderChannel {
    pub(crate) fn new(transaction: &HybridQrInitiatedTransaction, psk: &[u8; 32]) -> Self {
        let handshake = snow::Builder::new("Noise_KNpsk0_P256_AESGCM_SHA256".parse().unwrap())
            .prologue(&[1])
            .unwrap()
            .remote_public_key(&transaction.noise_public_key_sec1())
            .unwrap()
            .psk(0, psk)
            .unwrap()
            .build_responder()
            .unwrap();
        Self {
            handshake: Some(handshake),
            transport: None,
            incoming: VecDeque::new(),
        }
    }
}

impl HybridDataChannel for ResponderChannel {
    type Error = Infallible;

    async fn send(&mut self, message: Vec<u8>) -> Result<(), Self::Error> {
        let mut handshake = self.handshake.take().expect("one handshake message");
        let mut payload = [0u8; 1024];
        handshake.read_message(&message, &mut payload).unwrap();
        let mut response = vec![0u8; 1024];
        let length = handshake.write_message(&[], &mut response).unwrap();
        response.truncate(length);
        self.incoming.push_back(response);
        self.transport = Some(handshake.into_transport_mode().unwrap());
        Ok(())
    }

    async fn receive(&mut self) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.incoming.pop_front())
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
