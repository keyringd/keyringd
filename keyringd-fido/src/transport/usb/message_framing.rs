pub(crate) const INIT_HEADER_SIZE: usize = 7;
pub(crate) const CONTINUATION_HEADER_SIZE: usize = 5;
const MAX_CONTINUATION_PACKETS: usize = 128;

pub const fn maximum_message_size(packet_size: usize) -> Option<usize> {
    if packet_size < INIT_HEADER_SIZE {
        return None;
    }
    let framing_maximum =
        match MAX_CONTINUATION_PACKETS.checked_mul(packet_size - CONTINUATION_HEADER_SIZE) {
            Some(continuation) => match continuation.checked_add(packet_size - INIT_HEADER_SIZE) {
                Some(value) => value,
                None => usize::MAX,
            },
            None => usize::MAX,
        };
    Some(if framing_maximum < u16::MAX as usize {
        framing_maximum
    } else {
        u16::MAX as usize
    })
}
