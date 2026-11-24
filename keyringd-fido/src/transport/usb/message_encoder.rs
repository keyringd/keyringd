use super::CtapHidMessage;
use super::{CONTINUATION_HEADER_SIZE, INIT_HEADER_SIZE, TransportError, maximum_message_size};

pub fn encode_ctap_hid_message(
    message: &CtapHidMessage,
    packet_size: usize,
) -> Result<Vec<Vec<u8>>, TransportError> {
    let maximum = maximum_message_size(packet_size).ok_or(TransportError::PacketSizeTooSmall {
        minimum: INIT_HEADER_SIZE,
        actual: packet_size,
    })?;
    let payload = message.payload();
    if payload.len() > maximum {
        return Err(TransportError::MessageTooLarge {
            maximum,
            actual: payload.len(),
        });
    }

    let payload_length =
        u16::try_from(payload.len()).map_err(|_| TransportError::MessageTooLarge {
            maximum,
            actual: payload.len(),
        })?;
    let initial_payload_size = packet_size - INIT_HEADER_SIZE;
    let continuation_payload_size = packet_size - CONTINUATION_HEADER_SIZE;
    let channel = message.channel_id().to_be_bytes();
    let mut first = vec![0u8; packet_size];
    first[..4].copy_from_slice(&channel);
    first[4] = message.command().wire_value();
    first[5..INIT_HEADER_SIZE].copy_from_slice(&payload_length.to_be_bytes());
    let first_length = payload.len().min(initial_payload_size);
    first[INIT_HEADER_SIZE..INIT_HEADER_SIZE + first_length]
        .copy_from_slice(&payload[..first_length]);

    let remaining = &payload[first_length..];
    let continuation_count = remaining.len().div_ceil(continuation_payload_size);
    let mut packets = Vec::with_capacity(1 + continuation_count);
    packets.push(first);

    for (sequence, chunk) in remaining.chunks(continuation_payload_size).enumerate() {
        let mut packet = vec![0u8; packet_size];
        packet[..4].copy_from_slice(&channel);
        packet[4] = u8::try_from(sequence).map_err(|_| TransportError::SequenceOverflow)?;
        packet[CONTINUATION_HEADER_SIZE..CONTINUATION_HEADER_SIZE + chunk.len()]
            .copy_from_slice(chunk);
        packets.push(packet);
    }

    Ok(packets)
}
