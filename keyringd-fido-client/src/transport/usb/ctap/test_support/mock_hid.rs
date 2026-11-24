use crate::test_support::MockHidPacketIo;
use keyringd_fido::transport::usb::{CtapHidCommand, CtapHidMessageAssembler};
use std::collections::VecDeque;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

fn packet_io() -> MockHidPacketIo {
    let mut io = MockHidPacketIo::new();
    io.expect_input_packet_size().return_const(TEST_PACKET_SIZE);
    io.expect_output_packet_size()
        .return_const(TEST_PACKET_SIZE);
    io
}

pub(crate) fn fixed_response_hid(
    mut incoming: VecDeque<Vec<u8>>,
    writes: Arc<AtomicUsize>,
) -> MockHidPacketIo {
    let mut io = packet_io();
    io.expect_write_packet().returning(move |_| {
        writes.fetch_add(1, Ordering::Relaxed);
        Box::pin(async { Ok(()) })
    });
    io.expect_read_packet().returning(move || {
        let packet = incoming.pop_front().expect("fixed HID response");
        Box::pin(async move { Ok(packet) })
    });
    io
}

pub(crate) fn disconnected_hid(
    writes: Arc<AtomicUsize>,
    reads: Arc<AtomicUsize>,
) -> MockHidPacketIo {
    let mut io = packet_io();
    io.expect_write_packet().returning(move |_| {
        writes.fetch_add(1, Ordering::Relaxed);
        Box::pin(async { Ok(()) })
    });
    io.expect_read_packet().times(1).returning(move || {
        reads.fetch_add(1, Ordering::Relaxed);
        Box::pin(async {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "security key removed",
            ))
        })
    });
    io
}

pub(crate) fn pending_response_hid(
    mut request: CtapHidMessageAssembler,
    commands: Arc<Mutex<Vec<CtapHidCommand>>>,
) -> MockHidPacketIo {
    let mut io = packet_io();
    io.expect_write_packet().returning(move |packet| {
        if let Some(message) = request.push(packet).unwrap() {
            commands.lock().unwrap().push(message.command());
        }
        Box::pin(async { Ok(()) })
    });
    io.expect_read_packet()
        .returning(|| Box::pin(std::future::pending()));
    io
}

pub(crate) const TEST_PACKET_SIZE: usize = 64;
