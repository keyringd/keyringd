use crate::test_support::MockHidPacketIo;
use crate::transport::usb::ctap::CtapHidTransport;
use crate::transport::usb::ctap::test_support::TEST_PACKET_SIZE;
use crate::transport::usb::ctap::test_support::fixed_response_hid;
use keyringd_fido::transport::usb::CtapHidCapabilities;
use keyringd_fido::transport::usb::CtapHidInitResponse;
use keyringd_fido::transport::usb::CtapHidMessage;
use keyringd_fido::transport::usb::encode_ctap_hid_message;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

pub(crate) fn initialized_channel_with(
    responses: impl IntoIterator<Item = CtapHidMessage>,
) -> (CtapHidTransport<MockHidPacketIo>, Arc<AtomicUsize>) {
    let mut incoming = VecDeque::new();
    for response in responses {
        incoming.extend(encode_ctap_hid_message(&response, TEST_PACKET_SIZE).unwrap());
    }
    let writes = Arc::new(AtomicUsize::new(0));
    let mut channel = CtapHidTransport::new(fixed_response_hid(incoming, writes.clone()), None);
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
    (channel, writes)
}
