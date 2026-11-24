use super::CtapHidCommand;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CtapHidMessage {
    channel_id: u32,
    command: CtapHidCommand,
    payload: Vec<u8>,
}

impl CtapHidMessage {
    pub fn new(channel_id: u32, command: CtapHidCommand, payload: Vec<u8>) -> Self {
        Self {
            channel_id,
            command,
            payload,
        }
    }
    pub const fn channel_id(&self) -> u32 {
        self.channel_id
    }
    pub const fn command(&self) -> CtapHidCommand {
        self.command
    }
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
    pub fn into_payload(self) -> Vec<u8> {
        self.payload
    }
}
