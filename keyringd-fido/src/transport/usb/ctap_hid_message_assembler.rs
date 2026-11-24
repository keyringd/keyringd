use super::{CONTINUATION_HEADER_SIZE, INIT_HEADER_SIZE, TransportError, maximum_message_size};
use super::{CtapHidCommand, CtapHidMessage};

#[derive(Debug)]
pub struct CtapHidMessageAssembler {
    packet_size: usize,
    maximum_message_size: usize,
    partial: Option<PartialMessage>,
}

impl CtapHidMessageAssembler {
    pub fn new(packet_size: usize) -> Result<Self, TransportError> {
        let maximum_message_size =
            maximum_message_size(packet_size).ok_or(TransportError::PacketSizeTooSmall {
                minimum: INIT_HEADER_SIZE,
                actual: packet_size,
            })?;
        Ok(Self {
            packet_size,
            maximum_message_size,
            partial: None,
        })
    }

    pub fn reset(&mut self) {
        self.partial = None;
    }

    pub fn push(&mut self, packet: &[u8]) -> Result<Option<CtapHidMessage>, TransportError> {
        if packet.len() != self.packet_size {
            return Err(TransportError::InvalidPacketSize {
                expected: self.packet_size,
                actual: packet.len(),
            });
        }

        let channel_id = u32::from_be_bytes(packet[..4].try_into().expect("four-byte slice"));
        if packet[4] & 0x80 != 0 {
            return self.push_initialization(channel_id, packet);
        }
        self.push_continuation(channel_id, packet)
    }

    fn push_initialization(
        &mut self,
        channel_id: u32,
        packet: &[u8],
    ) -> Result<Option<CtapHidMessage>, TransportError> {
        if self.partial.is_some() {
            return Err(TransportError::UnexpectedInitialization);
        }
        let command = CtapHidCommand::from_wire(packet[4]);
        let expected_length = usize::from(u16::from_be_bytes([packet[5], packet[6]]));
        if expected_length > self.maximum_message_size {
            return Err(TransportError::MessageTooLarge {
                maximum: self.maximum_message_size,
                actual: expected_length,
            });
        }
        let first_length = expected_length.min(self.packet_size - INIT_HEADER_SIZE);
        let payload = packet[INIT_HEADER_SIZE..INIT_HEADER_SIZE + first_length].to_vec();
        if payload.len() == expected_length {
            return Ok(Some(CtapHidMessage::new(channel_id, command, payload)));
        }
        self.partial = Some(PartialMessage {
            channel_id,
            command,
            expected_length,
            next_sequence: 0,
            payload,
        });
        Ok(None)
    }

    fn push_continuation(
        &mut self,
        channel_id: u32,
        packet: &[u8],
    ) -> Result<Option<CtapHidMessage>, TransportError> {
        let partial = self
            .partial
            .as_mut()
            .ok_or(TransportError::UnexpectedContinuation)?;
        if partial.channel_id != channel_id {
            return Err(TransportError::ChannelChanged {
                expected: partial.channel_id,
                actual: channel_id,
            });
        }
        let sequence = packet[4];
        if sequence != partial.next_sequence {
            return Err(TransportError::InvalidSequence {
                expected: partial.next_sequence,
                actual: sequence,
            });
        }

        let remaining = partial.expected_length - partial.payload.len();
        let chunk_length = remaining.min(self.packet_size - CONTINUATION_HEADER_SIZE);
        partial.payload.extend_from_slice(
            &packet[CONTINUATION_HEADER_SIZE..CONTINUATION_HEADER_SIZE + chunk_length],
        );
        if partial.payload.len() == partial.expected_length {
            let complete = self.partial.take().expect("partial message exists");
            return Ok(Some(CtapHidMessage::new(
                complete.channel_id,
                complete.command,
                complete.payload,
            )));
        }
        partial.next_sequence = partial
            .next_sequence
            .checked_add(1)
            .ok_or(TransportError::SequenceOverflow)?;
        if partial.next_sequence & 0x80 != 0 {
            return Err(TransportError::SequenceOverflow);
        }
        Ok(None)
    }
}

use super::PartialMessage;
