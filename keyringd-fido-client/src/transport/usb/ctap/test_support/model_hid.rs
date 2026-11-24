use super::*;
use crate::usb::HidPacketIo;
use keyringd_fido::transport::usb::{
    CtapHidCapabilities, CtapHidCommand, CtapHidMessage, CtapHidMessageAssembler,
    encode_ctap_hid_message,
};
use keyringd_fido::{CtapCommand, CtapStatusCode};
use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};

/// Test HID I/O for reconfiguring cTaphiD requests to record commands and simulate the response.
pub(crate) struct ModelHid {
    pub(crate) incoming: VecDeque<Vec<u8>>,
    pub(crate) request: CtapHidMessageAssembler,
    pub(crate) commands: Arc<Mutex<Vec<CtapHidCommand>>>,
}

impl ModelHid {
    pub(crate) fn new() -> Self {
        Self {
            incoming: VecDeque::new(),
            request: CtapHidMessageAssembler::new(TEST_PACKET_SIZE).unwrap(),
            commands: Default::default(),
        }
    }

    fn enqueue(&mut self, message: CtapHidMessage) {
        self.incoming
            .extend(encode_ctap_hid_message(&message, TEST_PACKET_SIZE).unwrap());
    }
}

impl HidPacketIo for ModelHid {
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
                self.commands.lock().unwrap().push(message.command());
                match message.command() {
                    CtapHidCommand::Init => {
                        assert_eq!(message.channel_id(), 0xffff_ffff);
                        let mut response = message.payload().to_vec();
                        response.extend_from_slice(&0x0102_0304u32.to_be_bytes());
                        response.extend_from_slice(&[2, 1, 2, 3]);
                        response.push(CtapHidCapabilities::CBOR.bits());
                        response.extend_from_slice(&[0xaa, 0xbb]);
                        self.enqueue(CtapHidMessage::new(
                            0xffff_ffff,
                            CtapHidCommand::Init,
                            response,
                        ));
                    }
                    CtapHidCommand::Cbor => {
                        assert_eq!(message.payload(), &[CtapCommand::GetInfo.as_byte()]);
                        self.enqueue(CtapHidMessage::new(
                            0x0102_0304,
                            CtapHidCommand::Keepalive,
                            vec![1],
                        ));
                        self.enqueue(CtapHidMessage::new(
                            0x0102_0304,
                            CtapHidCommand::Keepalive,
                            vec![2],
                        ));
                        self.enqueue(CtapHidMessage::new(
                            0x0102_0304,
                            CtapHidCommand::Cbor,
                            vec![CtapStatusCode::Ok.as_byte(), 0xa0],
                        ));
                    }
                    CtapHidCommand::Cancel => {}
                    command => panic!("unexpected {command:?}"),
                }
            }
            Ok(())
        })
    }

    fn read_packet<'a>(
        &'a mut self,
    ) -> futures_util::future::BoxFuture<'a, Result<Vec<u8>, Self::Error>> {
        Box::pin(async move { Ok(self.incoming.pop_front().expect("model response")) })
    }
}
