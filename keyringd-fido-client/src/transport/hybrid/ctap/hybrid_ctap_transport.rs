// In order to align the definition file name to the type name, use the same name as the outer module.

use crate::{
    CtapTransport, HybridCtapTransportError, HybridDataChannel, HybridNoiseSession,
    HybridPlaintextMessage, TransportError,
};
use keyringd_fido::ctap::AuthenticatorGetInfoResponse;
use keyringd_fido::transport::hybrid::HybridPostHandshakeMessage;
use keyringd_fido::{CtapCommand, CtapRequest, CtapResponse, CtapStatusCode};

/// Combine Hybrid's data channel and Noise's cryptographic state to communicate with the smartphone's authenticator.
/// It maintains the GetInfo and the exit status received on connection, and handles encryption/decryption and termination notifications of CTAP messages.
pub struct HybridCtapTransport<D> {
    data_channel: D,
    noise: HybridNoiseSession,
    get_info: Vec<u8>,
    closed: bool,
}

impl<D> HybridCtapTransport<D>
where
    D: HybridDataChannel,
{
    pub(crate) async fn connect(
        mut data_channel: D,
        private_key: &[u8; 32],
        psk: &[u8; 32],
    ) -> Result<Self, HybridCtapTransportError<D::Error>> {
        let mut handshake = HybridNoiseSession::handshake(private_key, psk)?;
        let mut initial = vec![0; 1024];
        let length = handshake
            .write_message(&[], &mut initial)
            .map_err(crate::HybridNoiseError::from)?;
        initial.truncate(length);
        data_channel
            .send(initial)
            .await
            .map_err(HybridCtapTransportError::Channel)?;
        let response = data_channel
            .receive()
            .await
            .map_err(HybridCtapTransportError::Channel)?
            .ok_or(HybridCtapTransportError::ChannelClosed)?;
        let mut noise = HybridNoiseSession::from_handshake(handshake, &response)?;
        let encrypted = data_channel
            .receive()
            .await
            .map_err(HybridCtapTransportError::Channel)?
            .ok_or(HybridCtapTransportError::ChannelClosed)?;
        let initial = noise.decrypt(&encrypted)?;
        let get_info = decode_initial_message(&initial)?;
        AuthenticatorGetInfoResponse::from_cbor_bytes(&get_info)
            .map_err(HybridCtapTransportError::InvalidGetInfo)?;
        Ok(Self {
            data_channel,
            noise,
            get_info,
            closed: false,
        })
    }
}

impl<D> HybridCtapTransport<D>
where
    D: HybridDataChannel,
{
    pub(crate) async fn transmit(
        &mut self,
        request: &CtapRequest,
    ) -> Result<CtapResponse, HybridCtapTransportError<D::Error>> {
        self.ensure_open()?;
        let command = request.command();
        if command == CtapCommand::GetInfo {
            return Ok(CtapResponse::new(CtapStatusCode::Ok, self.get_info.clone()));
        }
        tracing::debug!(?command, "sending hybrid CTAP request");
        let encrypted = self.noise.encrypt_ctap(&request.to_bytes())?;
        self.data_channel
            .send(encrypted)
            .await
            .map_err(HybridCtapTransportError::Channel)?;
        let response = self.receive_ctap_response().await;
        match &response {
            Ok(response) => {
                tracing::debug!(?command, status = ?response.status(), "received hybrid CTAP response")
            }
            Err(error) => tracing::debug!(?command, %error, "hybrid CTAP response failed"),
        }
        response
    }

    pub(crate) async fn cancel(&mut self) -> Result<(), HybridCtapTransportError<D::Error>> {
        self.shutdown().await
    }

    pub(crate) async fn close(&mut self) -> Result<(), HybridCtapTransportError<D::Error>> {
        self.shutdown().await
    }
}
impl<D: HybridDataChannel> CtapTransport for HybridCtapTransport<D> {
    fn transmit<'a>(
        &'a mut self,
        request: &'a CtapRequest,
    ) -> futures_util::future::BoxFuture<'a, Result<CtapResponse, crate::CtapTransportError>> {
        Box::pin(async move {
            self.transmit(request)
                .await
                .map_err(crate::CtapTransportError::new)
        })
    }
    fn cancel(
        &mut self,
    ) -> futures_util::future::BoxFuture<'_, Result<(), crate::CtapTransportError>> {
        Box::pin(async move { self.cancel().await.map_err(crate::CtapTransportError::new) })
    }
    fn close(
        &mut self,
    ) -> futures_util::future::BoxFuture<'_, Result<(), crate::CtapTransportError>> {
        Box::pin(async move { self.close().await.map_err(crate::CtapTransportError::new) })
    }
}

impl<D> HybridCtapTransport<D>
where
    D: HybridDataChannel,
{
    async fn shutdown(&mut self) -> Result<(), HybridCtapTransportError<D::Error>> {
        self.ensure_open()?;
        self.closed = true;
        tracing::debug!("closing hybrid channel");
        let send_result = match self.noise.encrypt_shutdown() {
            Ok(shutdown) => self
                .data_channel
                .send(shutdown)
                .await
                .map_err(HybridCtapTransportError::Channel),
            Err(error) => Err(HybridCtapTransportError::Noise(error)),
        };
        let close_result = self
            .data_channel
            .close()
            .await
            .map_err(HybridCtapTransportError::Channel);
        send_result?;
        close_result?;
        tracing::debug!("hybrid channel closed");
        Ok(())
    }

    fn ensure_open(&self) -> Result<(), HybridCtapTransportError<D::Error>> {
        if self.closed {
            Err(HybridCtapTransportError::ChannelClosed)
        } else {
            Ok(())
        }
    }

    async fn receive_ctap_response(
        &mut self,
    ) -> Result<CtapResponse, HybridCtapTransportError<D::Error>> {
        loop {
            let encrypted = self
                .data_channel
                .receive()
                .await
                .map_err(HybridCtapTransportError::Channel)?
                .ok_or(HybridCtapTransportError::ChannelClosed)?;
            let plaintext = self.noise.decrypt(&encrypted)?;
            match HybridPlaintextMessage::from_bytes(&plaintext).map_err(map_frame_error)? {
                HybridPlaintextMessage::Shutdown => {
                    return Err(HybridCtapTransportError::UnexpectedShutdown);
                }
                HybridPlaintextMessage::Ctap(payload) => {
                    return CtapResponse::from_bytes(&payload)
                        .map_err(HybridCtapTransportError::InvalidCtapResponse);
                }
                HybridPlaintextMessage::Update(_) => {}
            }
        }
    }
}

fn map_frame_error<E>(error: TransportError) -> HybridCtapTransportError<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    match error {
        TransportError::InvalidHybridFrame => HybridCtapTransportError::InvalidFrame,
        TransportError::UnsupportedHybridMessageType(value) => {
            HybridCtapTransportError::UnsupportedMessageType(value)
        }
        _ => HybridCtapTransportError::InvalidFrame,
    }
}

fn decode_initial_message<E>(encoded: &[u8]) -> Result<Vec<u8>, HybridCtapTransportError<E>>
where
    E: std::error::Error + Send + Sync + 'static,
{
    let message = HybridPostHandshakeMessage::from_cbor_bytes(encoded)
        .map_err(HybridCtapTransportError::InvalidInitialMessage)?;
    if !message
        .supported_features()
        .iter()
        .any(|feature| feature == "ctap")
    {
        return Err(HybridCtapTransportError::CtapUnsupported);
    }
    Ok(message.get_info_response().to_vec())
}

#[cfg(test)]
mod tests {
    use crate::tests::support::transport_hybrid_ctap_hybrid_ctap_transport::get_info_bytes;

    use super::super::test_support::*;
    use super::*;
    use crate::{HybridAdvertisement, HybridOperationHint, HybridQrInitiatedTransaction};

    keyringd_testkit::scenario! {
        group "HybridCtapTransport::close" {
            case "Closed channel rejects reuse without sending another frame" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "fail a second send after closure and keep the frame count unchanged" {
                    let transaction = HybridQrInitiatedTransaction::new(HybridOperationHint::GetAssertion).unwrap();
                    let advertisement = HybridAdvertisement::new([0; 16]);
                    let model = ModelHybridChannel::new(&transaction, &advertisement, get_info_bytes());
                    let shutdown_frames = model.shutdown_frames.clone();
                    let mut channel = HybridCtapTransport::connect(
                        model,
                        &transaction.private_key_bytes(),
                        &transaction.psk(advertisement.plaintext()).unwrap(),
                    )
                    .await
                    .unwrap();
                    channel.close().await.unwrap();
                    assert!(matches!(
                        channel.cancel().await,
                        Err(HybridCtapTransportError::ChannelClosed)
                    ));
                    assert!(matches!(
                        channel
                            .transmit(&keyringd_fido::CtapRequest::new(
                                CtapCommand::GetInfo,
                                vec![]
                            ))
                            .await,
                        Err(HybridCtapTransportError::ChannelClosed)
                    ));
                    assert_eq!(shutdown_frames.load(std::sync::atomic::Ordering::SeqCst), 1);
                }
            }
        }
    }
}
