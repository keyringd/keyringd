#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CtapCommand {
    MakeCredential,
    GetAssertion,
    GetInfo,
    ClientPin,
    Reset,
    GetNextAssertion,
    BioEnrollment,
    CredentialManagement,
    Selection,
    LargeBlobs,
    Config,
    Unknown(u8),
}

impl CtapCommand {
    pub const fn from_byte(value: u8) -> Self {
        match value {
            0x01 => Self::MakeCredential,
            0x02 => Self::GetAssertion,
            0x04 => Self::GetInfo,
            0x06 => Self::ClientPin,
            0x07 => Self::Reset,
            0x08 => Self::GetNextAssertion,
            0x09 => Self::BioEnrollment,
            0x0a => Self::CredentialManagement,
            0x0b => Self::Selection,
            0x0c => Self::LargeBlobs,
            0x0d => Self::Config,
            value => Self::Unknown(value),
        }
    }

    pub const fn as_byte(self) -> u8 {
        match self {
            Self::MakeCredential => 0x01,
            Self::GetAssertion => 0x02,
            Self::GetInfo => 0x04,
            Self::ClientPin => 0x06,
            Self::Reset => 0x07,
            Self::GetNextAssertion => 0x08,
            Self::BioEnrollment => 0x09,
            Self::CredentialManagement => 0x0a,
            Self::Selection => 0x0b,
            Self::LargeBlobs => 0x0c,
            Self::Config => 0x0d,
            Self::Unknown(value) => value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "CtapCommand::from_byte" {
            case "Preserves unassigned CTAP command bytes" {
                why "Unknown command bytes must survive decoding for forward-compatible diagnostics without being treated as a command the client is allowed to generate."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#commands"
                given "an unassigned authenticator command byte" {
                    let byte = 0x7e;
                }
                when "the byte is decoded and encoded again" {
                    let command = CtapCommand::from_byte(byte);
                    let encoded = command.as_byte();
                }
                then "the unknown command byte remains available for forward compatibility" {
                    assert_eq!(command, CtapCommand::Unknown(byte));
                    assert_eq!(encoded, byte);
                }
            }
        }
    }
}
