use super::*;
use crate::usb::HidPacketIo;
use keyringd_fido::transport::usb::{
    CtapHidCapabilities, CtapHidCommand, CtapHidMessage, CtapHidMessageAssembler,
    encode_ctap_hid_message,
};
use std::collections::VecDeque;
use std::convert::Infallible;

/// Test HID I/O that reads the CTAPHID_INIT request and returns an initialization response in the specified format.
pub(crate) struct InitResponseHid {
    pub(crate) incoming: VecDeque<Vec<u8>>,
    pub(crate) request: CtapHidMessageAssembler,
    pub(crate) variant: InitResponseVariant,
}

impl InitResponseHid {
    pub(crate) fn new(variant: InitResponseVariant) -> Self {
        Self {
            incoming: VecDeque::new(),
            request: CtapHidMessageAssembler::new(TEST_PACKET_SIZE).unwrap(),
            variant,
        }
    }

    fn enqueue(&mut self, message: CtapHidMessage) {
        self.incoming
            .extend(encode_ctap_hid_message(&message, TEST_PACKET_SIZE).unwrap());
    }
}

impl HidPacketIo for InitResponseHid {
    type Error = Infallible;

    fn input_packet_size(&self) -> usize {
        TEST_PACKET_SIZE
    }

    fn output_packet_size(&self) -> usize {
        TEST_PACKET_SIZE
    }

    fn write_packet<'a>(
        &'a mut self,
        packet: &'a [u8],
    ) -> futures_util::future::BoxFuture<'a, Result<(), Self::Error>> {
        Box::pin(async move {
            if let Some(message) = self.request.push(packet).unwrap() {
                assert_eq!(message.command(), CtapHidCommand::Init);
                let mut nonce = message.payload().to_vec();
                let (channel_id, protocol_version, capabilities) = match self.variant {
                    InitResponseVariant::NonceMismatch => {
                        nonce[0] ^= 0xff;
                        (0x0102_0304, 2, CtapHidCapabilities::CBOR.bits())
                    }
                    InitResponseVariant::InvalidChannel(channel_id) => {
                        (channel_id, 2, CtapHidCapabilities::CBOR.bits())
                    }
                    InitResponseVariant::MissingCborCapability => (0x0102_0304, 2, 0),
                    InitResponseVariant::FutureProtocolVersion(protocol_version) => (
                        0x0102_0304,
                        protocol_version,
                        CtapHidCapabilities::CBOR.bits(),
                    ),
                };
                nonce.extend_from_slice(&channel_id.to_be_bytes());
                nonce.extend_from_slice(&[protocol_version, 1, 2, 3, capabilities]);
                self.enqueue(CtapHidMessage::new(
                    0xffff_ffff,
                    CtapHidCommand::Init,
                    nonce,
                ));
            }
            Ok(())
        })
    }

    fn read_packet<'a>(
        &'a mut self,
    ) -> futures_util::future::BoxFuture<'a, Result<Vec<u8>, Self::Error>> {
        Box::pin(async move { Ok(self.incoming.pop_front().expect("faulty INIT response")) })
    }
}
