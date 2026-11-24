use super::CtapCommand;

/// A request for sending a group of CTAP command identifiers and encoded payloads.
/// Command-specific CBOR content is not interpreted by the communication layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CtapRequest {
    command: CtapCommand,
    payload: Vec<u8>,
}

impl CtapRequest {
    pub fn new(command: CtapCommand, payload: Vec<u8>) -> Self {
        Self { command, payload }
    }
    pub const fn command(&self) -> CtapCommand {
        self.command
    }
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(1 + self.payload.len());
        encoded.push(self.command.as_byte());
        encoded.extend_from_slice(&self.payload);
        encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "CtapRequest::to_bytes" {
            case "Wire request prefixes payload with command" {
                verify "serialize the command byte immediately before the payload bytes" {
                    assert_eq!(
                        CtapRequest::new(CtapCommand::GetInfo, Vec::new()).to_bytes(),
                        [0x04]
                    );
                    assert_eq!(
                        CtapRequest::new(CtapCommand::MakeCredential, vec![0xa1, 0x01, 0x02]).to_bytes(),
                        [0x01, 0xa1, 0x01, 0x02]
                    );
                }
            }
        }
    }
}
