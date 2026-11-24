// In order to align the definition file name to the type name, use the same name as the outer module.

use keyringd_fido::{CtapRequest, CtapResponse};
use rand_core::{OsRng, RngCore};
use std::time::Duration;

use crate::{CtapTransport, CtapTransportStatus};
use keyringd_fido::TransportError;
use keyringd_fido::transport::usb::{
    CtapHidCapabilities, CtapHidCommand, CtapHidInitResponse, CtapHidMessage,
    CtapHidMessageAssembler, encode_ctap_hid_message,
};

use crate::usb::{CtapHidTransportError, HidPacketIo};

pub(crate) const BROADCAST_CHANNEL: u32 = 0xffff_ffff;
pub(crate) const CHANNEL_BUSY_ERROR: u8 = 0x06;
pub(crate) const CHANNEL_BUSY_RETRY_DELAY: Duration = Duration::from_millis(50);
pub(crate) const MAX_CHANNEL_BUSY_RETRIES: usize = 3;

/// A channel that communicates with the authenticator to CTAP using the HID packet I/O.
/// It deals with initialization of ctaphid, split/reconfiguration of messages, notification of progress, and cancellation. The notification destination will be specified at the time of construction.
pub struct CtapHidTransport<I> {
    io: I,
    pub(crate) init: Option<CtapHidInitResponse>,
    status_handler: Option<std::sync::Arc<dyn Fn(CtapTransportStatus) + Send + Sync>>,
}

impl<I> CtapHidTransport<I>
where
    I: HidPacketIo,
{
    pub fn new(
        io: I,
        status_handler: Option<std::sync::Arc<dyn Fn(CtapTransportStatus) + Send + Sync>>,
    ) -> Self {
        Self {
            io,
            init: None,
            status_handler,
        }
    }

    pub async fn initialize(
        &mut self,
    ) -> Result<&CtapHidInitResponse, CtapHidTransportError<I::Error>> {
        if self.init.is_none() {
            let mut nonce = [0u8; 8];
            OsRng.fill_bytes(&mut nonce);
            self.send_message(CtapHidMessage::new(
                BROADCAST_CHANNEL,
                CtapHidCommand::Init,
                nonce.to_vec(),
            ))
            .await?;
            let message = self.receive_message(BROADCAST_CHANNEL).await?;
            if message.command() != CtapHidCommand::Init {
                return Err(CtapHidTransportError::UnexpectedCommand {
                    expected: CtapHidCommand::Init,
                    actual: message.command(),
                });
            }
            let response = CtapHidInitResponse::parse(message.payload()).map_err(|error| {
                let TransportError::InvalidInitResponseLength { actual } = error else {
                    unreachable!("INIT response parser returned an unrelated transport error")
                };
                CtapHidTransportError::InvalidInitResponseLength(actual)
            })?;
            if response.nonce() != &nonce {
                return Err(CtapHidTransportError::InitNonceMismatch);
            }
            if matches!(response.channel_id(), 0 | BROADCAST_CHANNEL) {
                return Err(CtapHidTransportError::InvalidAllocatedChannel(
                    response.channel_id(),
                ));
            }
            if !response.capabilities().contains(CtapHidCapabilities::CBOR) {
                return Err(CtapHidTransportError::CborNotSupported);
            }
            self.init = Some(response);
        }
        Ok(self.init.as_ref().expect("initialized above"))
    }

    async fn send_message(
        &mut self,
        message: CtapHidMessage,
    ) -> Result<(), CtapHidTransportError<I::Error>> {
        for packet in encode_ctap_hid_message(&message, self.io.output_packet_size())? {
            self.io
                .write_packet(&packet)
                .await
                .map_err(CtapHidTransportError::Io)?;
        }
        Ok(())
    }

    async fn receive_message(
        &mut self,
        channel_id: u32,
    ) -> Result<CtapHidMessage, CtapHidTransportError<I::Error>> {
        let input_packet_size = self.io.input_packet_size();
        let mut assembler = CtapHidMessageAssembler::new(input_packet_size)?;
        loop {
            let packet = self
                .io
                .read_packet()
                .await
                .map_err(CtapHidTransportError::Io)?;
            if packet.len() != input_packet_size {
                return Err(TransportError::InvalidPacketSize {
                    expected: input_packet_size,
                    actual: packet.len(),
                }
                .into());
            }
            if packet_channel(&packet) != channel_id {
                continue;
            }
            if let Some(message) = assembler.push(&packet)? {
                return Ok(message);
            }
        }
    }
}

impl<I> CtapHidTransport<I>
where
    I: HidPacketIo,
{
    pub(crate) async fn transmit(
        &mut self,
        request: &CtapRequest,
    ) -> Result<CtapResponse, CtapHidTransportError<I::Error>> {
        let channel_id = self.initialize().await?.channel_id();
        let request = CtapHidMessage::new(channel_id, CtapHidCommand::Cbor, request.to_bytes());
        let mut channel_busy_retries = 0;

        loop {
            self.send_message(request.clone()).await?;
            loop {
                let message = self.receive_message(channel_id).await?;
                match message.command() {
                    CtapHidCommand::Keepalive => {
                        if message.payload().len() != 1 {
                            return Err(CtapHidTransportError::InvalidCommandPayloadLength {
                                command: CtapHidCommand::Keepalive,
                                expected: 1,
                                actual: message.payload().len(),
                            });
                        }
                        if let (Some(handler), Some(status)) = (
                            &self.status_handler,
                            message.payload().first().and_then(|status| match status {
                                1 => Some(CtapTransportStatus::Processing),
                                2 => Some(CtapTransportStatus::UserPresenceNeeded),
                                _ => None,
                            }),
                        ) {
                            handler(status);
                        }
                    }
                    CtapHidCommand::Cbor => {
                        return CtapResponse::from_bytes(message.payload()).map_err(Into::into);
                    }
                    CtapHidCommand::Error => {
                        if message.payload().len() != 1 {
                            return Err(CtapHidTransportError::InvalidCommandPayloadLength {
                                command: CtapHidCommand::Error,
                                expected: 1,
                                actual: message.payload().len(),
                            });
                        }
                        let error = message.payload()[0];
                        if error == CHANNEL_BUSY_ERROR
                            && channel_busy_retries < MAX_CHANNEL_BUSY_RETRIES
                        {
                            channel_busy_retries += 1;
                            tokio::time::sleep(CHANNEL_BUSY_RETRY_DELAY).await;
                            break;
                        }
                        return Err(CtapHidTransportError::Authenticator(error));
                    }
                    actual => {
                        return Err(CtapHidTransportError::UnexpectedCommand {
                            expected: CtapHidCommand::Cbor,
                            actual,
                        });
                    }
                }
            }
        }
    }

    pub(crate) async fn cancel(&mut self) -> Result<(), CtapHidTransportError<I::Error>> {
        if let Some(channel_id) = self.init.take().map(|init| init.channel_id()) {
            self.send_message(CtapHidMessage::new(
                channel_id,
                CtapHidCommand::Cancel,
                Vec::new(),
            ))
            .await?;
        }
        Ok(())
    }
}
impl<I: HidPacketIo> CtapTransport for CtapHidTransport<I> {
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
}

fn packet_channel(packet: &[u8]) -> u32 {
    u32::from_be_bytes(packet[..4].try_into().expect("four-byte slice"))
}

#[cfg(test)]
mod tests {

    use super::super::test_support::*;

    use keyringd_fido::CtapCommand;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    keyringd_testkit::scenario! {
        group "CtapHidTransport::transmit" {
            case "Reports disconnect errors and releases packet I/O" {
                why "A device disconnect during packet I/O must release the in-flight I/O future and report the loss instead of leaving a task or handle blocked."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-message-and-packet-structure"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                given "Return disconnect error on READ" {
                    let writes = Arc::new(AtomicUsize::new(0));
                    let reads = Arc::new(AtomicUsize::new(0));
                    let mut channel = CtapHidTransport::new(disconnected_hid(writes.clone(), reads.clone()), None);
                }
                when "Call CtapHidInitResponse::parse" {
                    channel.init = Some(
                        CtapHidInitResponse::parse(&[
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            1,
                            2,
                            3,
                            4,
                            2,
                            1,
                            0,
                            0,
                            CtapHidCapabilities::CBOR.bits(),
                        ])
                        .unwrap(),
                    );
                }
                then "Verify that the results match CtapHidChannelError::Io and ErrorKind::NotConnected" {
                    assert!(matches!(
                        channel.transmit(&keyringd_fido::CtapRequest::new(CtapCommand::GetInfo, Vec::new())).await,
                        Err(CtapHidTransportError::Io(error))
                            if error.kind() == std::io::ErrorKind::NotConnected
                    ));
                    assert_eq!(writes.load(Ordering::Relaxed), 1);
                    assert_eq!(reads.load(Ordering::Relaxed), 1);
                }
            }
        }
    }
}
