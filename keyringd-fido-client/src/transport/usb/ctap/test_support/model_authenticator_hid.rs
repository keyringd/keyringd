use super::*;
use crate::usb::HidPacketIo;
use keyringd_fido::CtapCommand;
use keyringd_fido::transport::usb::{
    CtapHidCapabilities, CtapHidCommand, CtapHidMessage, CtapHidMessageAssembler,
    encode_ctap_hid_message,
};
use keyringd_test_authenticator::ModelCtapAuthenticator;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::Arc;

/// Test input/output that reconfigures HID packets into CTAP requests, passes them to the authenticator model, and returns a response as HID packet.
pub(crate) struct ModelAuthenticatorHid {
    pub(crate) authenticator: Arc<ModelCtapAuthenticator>,
    pub(crate) incoming: VecDeque<Vec<u8>>,
    pub(crate) request: CtapHidMessageAssembler,
}

impl ModelAuthenticatorHid {
    pub(crate) fn new() -> Self {
        Self {
            authenticator: Arc::new(ModelCtapAuthenticator::new()),
            incoming: VecDeque::new(),
            request: CtapHidMessageAssembler::new(TEST_PACKET_SIZE).unwrap(),
        }
    }

    fn enqueue(&mut self, message: CtapHidMessage) {
        self.incoming
            .extend(encode_ctap_hid_message(&message, TEST_PACKET_SIZE).unwrap());
    }
}

impl HidPacketIo for ModelAuthenticatorHid {
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
                match message.command() {
                    CtapHidCommand::Init => {
                        assert_eq!(message.channel_id(), 0xffff_ffff);
                        let mut response = message.payload().to_vec();
                        response.extend_from_slice(&0x0102_0304u32.to_be_bytes());
                        response.extend_from_slice(&[2, 1, 2, 3]);
                        response.push(CtapHidCapabilities::CBOR.bits());
                        self.enqueue(CtapHidMessage::new(
                            0xffff_ffff,
                            CtapHidCommand::Init,
                            response,
                        ));
                    }
                    CtapHidCommand::Cbor => {
                        let (&command, request) = message
                            .payload()
                            .split_first()
                            .expect("CTAPHID CBOR payload contains a command byte");
                        let response = self
                            .authenticator
                            .transmit(CtapCommand::from_byte(command), request)
                            .expect("model authenticator response");
                        self.enqueue(CtapHidMessage::new(
                            0x0102_0304,
                            CtapHidCommand::Cbor,
                            response,
                        ));
                    }
                    CtapHidCommand::Cancel => self.authenticator.cancel(),
                    command => panic!("unexpected {command:?}"),
                }
            }
            Ok(())
        })
    }

    fn read_packet<'a>(
        &'a mut self,
    ) -> futures_util::future::BoxFuture<'a, Result<Vec<u8>, Self::Error>> {
        Box::pin(async move {
            Ok(self
                .incoming
                .pop_front()
                .expect("model authenticator HID response"))
        })
    }
}
