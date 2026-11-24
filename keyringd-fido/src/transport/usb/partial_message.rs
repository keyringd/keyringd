use super::CtapHidCommand;

#[derive(Debug)]
pub(super) struct PartialMessage {
    pub(super) channel_id: u32,
    pub(super) command: CtapHidCommand,
    pub(super) expected_length: usize,
    pub(super) next_sequence: u8,
    pub(super) payload: Vec<u8>,
}
